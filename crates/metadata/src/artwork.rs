use std::io::Cursor;

use domain::error::ArtworkError;
use domain::metadata::{ArtworkFormat, ArtworkPipeline, ArtworkSpec, ProcessedArtwork};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat};

const JPEG_QUALITY: u8 = 82;

pub struct ImageArtworkPipeline {
    client: reqwest::Client,
}

impl ImageArtworkPipeline {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for ImageArtworkPipeline {
    fn default() -> Self {
        Self::new()
    }
}

fn needs_alpha(image: &DynamicImage) -> bool {
    image.color().has_alpha() && image.to_rgba8().pixels().any(|pixel| pixel.0[3] < u8::MAX)
}

fn encode(
    image: &DynamicImage,
    spec: ArtworkSpec,
    format: ArtworkFormat,
) -> Result<ProcessedArtwork, ArtworkError> {
    let resized = image.thumbnail(spec.max_width, spec.max_height);
    let mut bytes = Vec::new();
    match format {
        ArtworkFormat::Jpeg => JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY)
            .encode_image(&resized.to_rgb8())
            .map_err(|e| ArtworkError::Decode(e.to_string()))?,
        ArtworkFormat::Png => resized
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .map_err(|e| ArtworkError::Decode(e.to_string()))?,
    }
    Ok(ProcessedArtwork {
        width: resized.width(),
        height: resized.height(),
        bytes,
        format,
    })
}

impl ImageArtworkPipeline {
    async fn download(&self, url: &str) -> Result<Vec<u8>, ArtworkError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ArtworkError::Download(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ArtworkError::Download(format!(
                "http status {}",
                response.status()
            )));
        }
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|e| ArtworkError::Download(e.to_string()))
    }
}

impl ArtworkPipeline for ImageArtworkPipeline {
    async fn process_all(
        &self,
        url: &str,
        specs: &[ArtworkSpec],
    ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
        let bytes = self.download(url).await?;
        let specs = specs.to_vec();
        tokio::task::spawn_blocking(move || {
            let image =
                image::load_from_memory(&bytes).map_err(|e| ArtworkError::Decode(e.to_string()))?;
            let format = if needs_alpha(&image) {
                ArtworkFormat::Png
            } else {
                ArtworkFormat::Jpeg
            };
            specs
                .into_iter()
                .map(|spec| encode(&image, spec, format))
                .collect()
        })
        .await
        .map_err(|e| ArtworkError::Decode(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, Rgba};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const RUNGS: [ArtworkSpec; 3] = [
        ArtworkSpec {
            max_width: 180,
            max_height: 540,
        },
        ArtworkSpec {
            max_width: 480,
            max_height: 1440,
        },
        ArtworkSpec {
            max_width: 960,
            max_height: 2880,
        },
    ];

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let buffer = ImageBuffer::from_pixel(width, height, Rgb([200u8, 50, 25]));
        let image = DynamicImage::ImageRgb8(buffer);
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn transparent_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let buffer = ImageBuffer::from_pixel(width, height, Rgba([200u8, 50, 25, 0]));
        let image = DynamicImage::ImageRgba8(buffer);
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn opaque_rgba_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let buffer = ImageBuffer::from_pixel(width, height, Rgba([200u8, 50, 25, 255]));
        let image = DynamicImage::ImageRgba8(buffer);
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn one_spec(max_width: u32, max_height: u32) -> [ArtworkSpec; 1] {
        [ArtworkSpec {
            max_width,
            max_height,
        }]
    }

    async fn serve(body: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(body)
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn downloads_and_resizes_preserving_aspect() {
        let server = serve(ResponseTemplate::new(200).set_body_bytes(png_bytes(200, 300))).await;
        let url = format!("{}/poster.png", server.uri());
        let art = ImageArtworkPipeline::new()
            .process_all(&url, &one_spec(100, 100))
            .await
            .unwrap()
            .remove(0);
        assert!(art.width <= 100 && art.height <= 100);
        assert!(art.width > 0 && art.height > 0);
        assert!((art.width, art.height) != (200, 300));
        let decoded = image::load_from_memory(&art.bytes).unwrap();
        assert_eq!(decoded.width(), art.width);
        assert_eq!(decoded.height(), art.height);
    }

    #[tokio::test]
    async fn every_rung_is_rendered_from_a_single_request() {
        let server = serve(ResponseTemplate::new(200).set_body_bytes(png_bytes(1000, 1500))).await;
        let url = format!("{}/poster.png", server.uri());

        let rendered = ImageArtworkPipeline::new()
            .process_all(&url, &RUNGS)
            .await
            .unwrap();

        assert_eq!(rendered.len(), RUNGS.len());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        for (art, spec) in rendered.iter().zip(RUNGS) {
            assert!(art.width <= spec.max_width);
            assert_eq!(
                image::load_from_memory(&art.bytes).unwrap().width(),
                art.width
            );
        }
    }

    #[tokio::test]
    async fn a_photographic_source_is_stored_as_jpeg() {
        let server = serve(ResponseTemplate::new(200).set_body_bytes(png_bytes(400, 600))).await;
        let url = format!("{}/poster.png", server.uri());

        let art = ImageArtworkPipeline::new()
            .process_all(&url, &one_spec(200, 600))
            .await
            .unwrap()
            .remove(0);

        assert_eq!(art.format, ArtworkFormat::Jpeg);
        assert_eq!(&art.bytes[..3], &[0xFF, 0xD8, 0xFF]);
    }

    #[tokio::test]
    async fn a_transparent_source_keeps_its_alpha_as_png() {
        let server =
            serve(ResponseTemplate::new(200).set_body_bytes(transparent_png_bytes(400, 600))).await;
        let url = format!("{}/logo.png", server.uri());

        let art = ImageArtworkPipeline::new()
            .process_all(&url, &one_spec(200, 600))
            .await
            .unwrap()
            .remove(0);

        assert_eq!(art.format, ArtworkFormat::Png);
        assert!(
            image::load_from_memory(&art.bytes)
                .unwrap()
                .color()
                .has_alpha()
        );
    }

    #[tokio::test]
    async fn an_alpha_channel_that_is_fully_opaque_still_becomes_jpeg() {
        let server =
            serve(ResponseTemplate::new(200).set_body_bytes(opaque_rgba_png_bytes(400, 600))).await;
        let url = format!("{}/poster.png", server.uri());

        let art = ImageArtworkPipeline::new()
            .process_all(&url, &one_spec(200, 600))
            .await
            .unwrap()
            .remove(0);

        assert_eq!(art.format, ArtworkFormat::Jpeg);
    }

    #[tokio::test]
    async fn non_success_status_is_download_error() {
        let server = serve(ResponseTemplate::new(404)).await;
        let url = format!("{}/missing.png", server.uri());
        let result = ImageArtworkPipeline::default()
            .process_all(&url, &one_spec(50, 50))
            .await;
        assert!(matches!(result, Err(ArtworkError::Download(_))));
    }

    #[tokio::test]
    async fn non_image_body_is_decode_error() {
        let server = serve(ResponseTemplate::new(200).set_body_string("not an image")).await;
        let url = format!("{}/bad.png", server.uri());
        let result = ImageArtworkPipeline::new()
            .process_all(&url, &one_spec(50, 50))
            .await;
        assert!(matches!(result, Err(ArtworkError::Decode(_))));
    }

    #[tokio::test]
    async fn transport_failure_is_download_error() {
        let result = ImageArtworkPipeline::new()
            .process_all("http://127.0.0.1:1/x.png", &one_spec(50, 50))
            .await;
        assert!(matches!(result, Err(ArtworkError::Download(_))));
    }
}
