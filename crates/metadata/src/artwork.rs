use std::io::Cursor;

use domain::error::ArtworkError;
use domain::metadata::{ArtworkPipeline, ArtworkSpec, ProcessedArtwork};
use image::ImageFormat;

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

impl ArtworkPipeline for ImageArtworkPipeline {
    async fn process(
        &self,
        url: &str,
        spec: ArtworkSpec,
    ) -> Result<ProcessedArtwork, ArtworkError> {
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
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ArtworkError::Download(e.to_string()))?;
        let image =
            image::load_from_memory(&bytes).map_err(|e| ArtworkError::Decode(e.to_string()))?;
        let resized = image.thumbnail(spec.max_width, spec.max_height);
        let mut buffer = Cursor::new(Vec::new());
        resized
            .write_to(&mut buffer, ImageFormat::Png)
            .map_err(|e| ArtworkError::Decode(e.to_string()))?;
        Ok(ProcessedArtwork {
            width: resized.width(),
            height: resized.height(),
            bytes: buffer.into_inner(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, Rgb};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let buffer = ImageBuffer::from_pixel(width, height, Rgb([200u8, 50, 25]));
        let image = DynamicImage::ImageRgb8(buffer);
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
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
        let spec = ArtworkSpec {
            max_width: 100,
            max_height: 100,
        };
        let art = ImageArtworkPipeline::new()
            .process(&url, spec)
            .await
            .unwrap();
        assert!(art.width <= 100 && art.height <= 100);
        assert!(art.width > 0 && art.height > 0);
        assert!((art.width, art.height) != (200, 300));
        let decoded = image::load_from_memory(&art.bytes).unwrap();
        assert_eq!(decoded.width(), art.width);
        assert_eq!(decoded.height(), art.height);
    }

    #[tokio::test]
    async fn non_success_status_is_download_error() {
        let server = serve(ResponseTemplate::new(404)).await;
        let url = format!("{}/missing.png", server.uri());
        let result = ImageArtworkPipeline::default()
            .process(
                &url,
                ArtworkSpec {
                    max_width: 50,
                    max_height: 50,
                },
            )
            .await;
        assert!(matches!(result, Err(ArtworkError::Download(_))));
    }

    #[tokio::test]
    async fn non_image_body_is_decode_error() {
        let server = serve(ResponseTemplate::new(200).set_body_string("not an image")).await;
        let url = format!("{}/bad.png", server.uri());
        let result = ImageArtworkPipeline::new()
            .process(
                &url,
                ArtworkSpec {
                    max_width: 50,
                    max_height: 50,
                },
            )
            .await;
        assert!(matches!(result, Err(ArtworkError::Decode(_))));
    }

    #[tokio::test]
    async fn transport_failure_is_download_error() {
        let result = ImageArtworkPipeline::new()
            .process(
                "http://127.0.0.1:1/x.png",
                ArtworkSpec {
                    max_width: 50,
                    max_height: 50,
                },
            )
            .await;
        assert!(matches!(result, Err(ArtworkError::Download(_))));
    }
}
