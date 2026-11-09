use std::io::Cursor;

use domain::error::ArtworkError;
use domain::metadata::{ArtworkFormat, ArtworkPipeline, ArtworkSpec, ProcessedArtwork};
use http_body_util::{BodyExt, Limited};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat};

const JPEG_QUALITY: u8 = 82;
const MAX_THUMBNAIL_BYTES: usize = 20 * 1024 * 1024;

pub struct ImageArtworkPipeline {
    client: reqwest::Client,
}

impl ImageArtworkPipeline {
    pub fn new() -> Self {
        Self { client: crate::http::client() }
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
    Ok(ProcessedArtwork { width: resized.width(), height: resized.height(), bytes, format })
}

async fn render(
    bytes: Vec<u8>,
    specs: &[ArtworkSpec],
) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
    let specs = specs.to_vec();
    tokio::task::spawn_blocking(move || {
        let image =
            image::load_from_memory(&bytes).map_err(|e| ArtworkError::Decode(e.to_string()))?;
        let format = if needs_alpha(&image) { ArtworkFormat::Png } else { ArtworkFormat::Jpeg };
        specs.into_iter().map(|spec| encode(&image, spec, format)).collect()
    })
    .await
    .map_err(|e| ArtworkError::Decode(e.to_string()))?
}

impl ImageArtworkPipeline {
    async fn download_at_most(&self, url: &str, limit: usize) -> Result<Vec<u8>, ArtworkError> {
        let response =
            self.client.get(url).send().await.map_err(|e| ArtworkError::Download(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ArtworkError::Download(format!("http status {}", response.status())));
        }
        Limited::new(reqwest::Body::from(response), limit)
            .collect()
            .await
            .map(|body| body.to_bytes().to_vec())
            .map_err(|e| ArtworkError::Download(e.to_string()))
    }
}

impl ArtworkPipeline for ImageArtworkPipeline {
    async fn process_all(
        &self,
        url: &str,
        specs: &[ArtworkSpec],
    ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
        render(self.download_at_most(url, usize::MAX).await?, specs).await
    }

    async fn process_jpeg(
        &self,
        url: &str,
        specs: &[ArtworkSpec],
    ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
        let bytes = self.download_at_most(url, MAX_THUMBNAIL_BYTES).await?;
        if image::guess_format(&bytes).ok() != Some(ImageFormat::Jpeg) {
            return Err(ArtworkError::Decode(format!("the thumbnail at [{url}] is not a JPEG")));
        }
        render(bytes, specs).await
    }

    async fn process_bytes(
        &self,
        bytes: Vec<u8>,
        specs: &[ArtworkSpec],
    ) -> Result<Vec<ProcessedArtwork>, ArtworkError> {
        render(bytes, specs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, Rgba};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const RUNGS: [ArtworkSpec; 3] = [
        ArtworkSpec { max_width: 180, max_height: 540 },
        ArtworkSpec { max_width: 480, max_height: 1440 },
        ArtworkSpec { max_width: 960, max_height: 2880 },
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

    fn jpeg_bytes(width: u32, height: u32) -> Vec<u8> {
        let buffer = ImageBuffer::from_pixel(width, height, Rgb([200u8, 50, 25]));
        let mut out = Vec::new();
        JpegEncoder::new_with_quality(&mut out, 90).encode_image(&buffer).unwrap();
        out
    }

    fn webp_bytes() -> Vec<u8> {
        let mut out = b"RIFF\x1a\x00\x00\x00WEBPVP8L\x0d\x00\x00\x00".to_vec();
        out.extend([0x2f, 0, 0, 0, 0x10, 0x07, 0x10, 0x11, 0x11, 0x88, 0x88, 0xfe, 0x07]);
        out
    }

    fn one_spec(max_width: u32, max_height: u32) -> [ArtworkSpec; 1] {
        [ArtworkSpec { max_width, max_height }]
    }

    async fn serve(body: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET")).respond_with(body).mount(&server).await;
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

        let rendered = ImageArtworkPipeline::new().process_all(&url, &RUNGS).await.unwrap();

        assert_eq!(rendered.len(), RUNGS.len());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        for (art, spec) in rendered.iter().zip(RUNGS) {
            assert!(art.width <= spec.max_width);
            assert_eq!(image::load_from_memory(&art.bytes).unwrap().width(), art.width);
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
        assert!(image::load_from_memory(&art.bytes).unwrap().color().has_alpha());
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
        let result = ImageArtworkPipeline::default().process_all(&url, &one_spec(50, 50)).await;
        assert!(matches!(result, Err(ArtworkError::Download(_))));
    }

    #[tokio::test]
    async fn non_image_body_is_decode_error() {
        let server = serve(ResponseTemplate::new(200).set_body_string("not an image")).await;
        let url = format!("{}/bad.png", server.uri());
        let result = ImageArtworkPipeline::new().process_all(&url, &one_spec(50, 50)).await;
        assert!(matches!(result, Err(ArtworkError::Decode(_))));
    }

    #[tokio::test]
    async fn a_jpeg_thumbnail_from_a_url_without_an_extension_renders_every_rung() {
        let server = serve(ResponseTemplate::new(200).set_body_bytes(jpeg_bytes(1280, 720))).await;
        let url = format!("{}/video/x8abc/thumbnail", server.uri());

        let rendered = ImageArtworkPipeline::new().process_jpeg(&url, &RUNGS).await.unwrap();

        assert_eq!(rendered.len(), RUNGS.len());
        for (art, spec) in rendered.iter().zip(RUNGS) {
            assert!(art.width <= spec.max_width);
            assert_eq!(art.format, ArtworkFormat::Jpeg);
        }
    }

    #[tokio::test]
    async fn a_png_thumbnail_is_refused() {
        let server = serve(ResponseTemplate::new(200).set_body_bytes(png_bytes(400, 300))).await;
        let url = format!("{}/thumb.jpg", server.uri());
        let result = ImageArtworkPipeline::new().process_jpeg(&url, &one_spec(50, 50)).await;
        assert!(matches!(result, Err(ArtworkError::Decode(_))));
    }

    #[tokio::test]
    async fn a_webp_thumbnail_is_refused() {
        let server = serve(ResponseTemplate::new(200).set_body_bytes(webp_bytes())).await;
        let url = format!("{}/thumb.webp", server.uri());
        let result = ImageArtworkPipeline::new().process_jpeg(&url, &one_spec(50, 50)).await;
        assert!(matches!(result, Err(ArtworkError::Decode(_))));
    }

    #[tokio::test]
    async fn a_thumbnail_that_cannot_be_downloaded_is_a_download_error() {
        let server = serve(ResponseTemplate::new(404)).await;
        let url = format!("{}/thumb.jpg", server.uri());
        let result = ImageArtworkPipeline::new().process_jpeg(&url, &one_spec(50, 50)).await;
        assert!(matches!(result, Err(ArtworkError::Download(_))));
    }

    #[tokio::test]
    async fn a_body_over_the_limit_is_a_download_error() {
        let server = serve(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 64])).await;
        let url = format!("{}/thumb.jpg", server.uri());
        let pipeline = ImageArtworkPipeline::new();
        assert_eq!(pipeline.download_at_most(&url, 64).await.unwrap().len(), 64);
        assert!(matches!(
            pipeline.download_at_most(&url, 63).await,
            Err(ArtworkError::Download(_))
        ));
    }

    #[tokio::test]
    async fn a_thumbnail_over_the_limit_is_a_download_error() {
        let mut body = jpeg_bytes(16, 16);
        body.resize(MAX_THUMBNAIL_BYTES + 1, 0);
        let server = serve(ResponseTemplate::new(200).set_body_bytes(body)).await;
        let url = format!("{}/thumb.jpg", server.uri());
        let result = ImageArtworkPipeline::new().process_jpeg(&url, &one_spec(50, 50)).await;
        assert!(matches!(result, Err(ArtworkError::Download(_))));
    }

    #[test]
    fn the_thumbnail_limit_is_twenty_megabytes() {
        assert_eq!(MAX_THUMBNAIL_BYTES, 20 * 1024 * 1024);
    }

    #[tokio::test]
    async fn bytes_render_every_rung_without_a_request() {
        let rendered = ImageArtworkPipeline::new()
            .process_bytes(jpeg_bytes(1920, 1080), &RUNGS)
            .await
            .unwrap();

        assert_eq!(rendered.len(), RUNGS.len());
        for (art, spec) in rendered.iter().zip(RUNGS) {
            assert!(art.width <= spec.max_width);
            assert_eq!(image::load_from_memory(&art.bytes).unwrap().width(), art.width);
        }
    }

    #[tokio::test]
    async fn bytes_that_are_not_an_image_are_a_decode_error() {
        let result =
            ImageArtworkPipeline::new().process_bytes(b"not an image".to_vec(), &RUNGS).await;
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
