use super::*;
use crate::metadata::{Bucket, Candidate, ContentType, MetadataFields, ScoredCandidate};
use image::{DynamicImage, GenericImageView, ImageFormat, Rgb, RgbImage};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

fn png(image: RgbImage) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

#[test]
fn t1_recorte_central_preserva_quadrado_vermelho() {
    let mut image = RgbImage::from_pixel(1280, 720, Rgb([0, 0, 255]));
    for y in 0..720 {
        for x in 280..1000 {
            image.put_pixel(x, y, Rgb([255, 0, 0]));
        }
    }
    let jpeg = process(&png(image)).unwrap();
    assert_eq!(image::guess_format(&jpeg).unwrap(), ImageFormat::Jpeg);
    let decoded = image::load_from_memory(&jpeg).unwrap().to_rgb8();
    assert_eq!(decoded.dimensions(), (720, 720));
    for (x, y) in [(0, 0), (360, 360), (719, 719)] {
        let pixel = decoded.get_pixel(x, y);
        assert!(pixel[0] >= 250 && pixel[1] < 5 && pixel[2] < 5, "{pixel:?}");
    }
}

#[test]
fn t1_redimensiona_3000_para_1200() {
    let jpeg = process(&png(RgbImage::from_pixel(3000, 3000, Rgb([255, 0, 0])))).unwrap();
    assert_eq!(
        image::load_from_memory(&jpeg).unwrap().dimensions(),
        (1200, 1200)
    );
}

#[test]
fn recorta_retrato_e_nao_amplia_imagem_pequena() {
    let jpeg = process(&png(RgbImage::new(80, 160))).unwrap();
    assert_eq!(
        image::load_from_memory(&jpeg).unwrap().dimensions(),
        (80, 80)
    );
    let jpeg = process(&png(RgbImage::new(104, 100))).unwrap();
    assert_eq!(
        image::load_from_memory(&jpeg).unwrap().dimensions(),
        (104, 100)
    );
}

#[test]
fn aceita_jpeg_e_webp() {
    for format in [ImageFormat::Jpeg, ImageFormat::WebP] {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::new(25, 25))
            .write_to(&mut bytes, format)
            .unwrap();
        let jpeg = process(bytes.get_ref()).unwrap();
        assert_eq!(image::guess_format(&jpeg).unwrap(), ImageFormat::Jpeg);
    }
}

#[test]
fn rejeita_dados_invalidos_e_limite_excedido() {
    assert_eq!(process(b"invalid").unwrap_err().kind(), "artwork_decode");
    assert_eq!(
        process(&vec![0; MAX_DOWNLOAD_BYTES + 1])
            .unwrap_err()
            .kind(),
        "artwork_size"
    );
}

fn metadata() -> MetadataResult {
    MetadataResult {
        fields: MetadataFields::default(),
        confidence: 1.0,
        source: "youtube_music".into(),
        bucket: Bucket::Auto,
        candidates: Vec::new(),
        content_type: ContentType::Music,
        isrc: None,
        official: None,
    }
}

fn candidate(provider: &str, url: &str, score: f64) -> ScoredCandidate {
    let mut candidate = Candidate::new(provider, url, "Song");
    candidate.cover_url = Some(url.into());
    ScoredCandidate { candidate, score }
}

#[test]
fn ordem_aplicado_demais_por_provedor_e_youtube_sem_duplicatas() {
    let mut result = metadata();
    result.source = "itunes".into();
    result.fields.cover_url = Some("applied".into());
    result.candidates = vec![
        candidate("musicbrainz", "caa", 0.9),
        candidate("itunes", "applied", 1.0),
        candidate("itunes", "itunes2", 0.95),
        candidate("deezer", "low", 0.84),
        candidate("deezer", "deezer", 0.92),
        candidate("deezer", "deezer", 0.91),
    ];
    let covers = candidates(&result, Some("thumbnail"), 0.85);
    assert_eq!(
        covers.iter().map(|c| c.url.as_str()).collect::<Vec<_>>(),
        ["applied", "deezer", "itunes2", "caa", "thumbnail"]
    );
    assert_eq!(
        covers.iter().map(|c| c.source.as_str()).collect::<Vec<_>>(),
        ["itunes", "deezer", "itunes", "musicbrainz", "youtube"]
    );
}

#[test]
fn oficial_so_usa_complemento_confiavel_e_user_tem_prioridade() {
    let mut result = metadata();
    result.fields.cover_url = Some("itunes".into());
    result.candidates = vec![
        candidate("itunes", "itunes", 0.9),
        candidate("deezer", "deezer", 0.95),
    ];
    assert_eq!(
        candidates(&result, Some("thumbnail"), 0.85)[0].source,
        "itunes"
    );
    result.candidates[0].score = 0.5;
    assert_eq!(
        candidates(&result, Some("thumbnail"), 0.85)[0].source,
        "deezer"
    );
    result.source = "user".into();
    result.fields.cover_url = Some("custom".into());
    assert_eq!(candidates(&result, None, 0.85)[0].url, "custom");
}

#[tokio::test]
async fn t1_invalido_e_http_falhou_tenta_proximo_e_todos_falham_none() {
    let server = MockServer::start().await;
    for (url, response) in [
        (
            "/invalid",
            ResponseTemplate::new(200).set_body_bytes(b"invalid"),
        ),
        ("/http", ResponseTemplate::new(503)),
        (
            "/ok",
            ResponseTemplate::new(200).set_body_bytes(png(RgbImage::new(60, 80))),
        ),
    ] {
        Mock::given(path(url))
            .respond_with(response)
            .mount(&server)
            .await;
    }
    let covers: Vec<_> = ["invalid", "http", "ok"]
        .into_iter()
        .map(|p| CoverCandidate {
            url: format!("{}/{p}", server.uri()),
            source: p.into(),
        })
        .collect();
    let client = ArtworkClient::default();
    let cover = client.fetch(&covers).await.unwrap();
    assert_eq!(cover.cover_source, "ok");
    assert_eq!(
        image::load_from_memory(&cover.jpeg).unwrap().dimensions(),
        (60, 60)
    );
    assert!(client.fetch(&covers[..2]).await.is_none());
    assert!(client.fetch(&[]).await.is_none());
}

#[tokio::test]
async fn download_rejeita_content_length_acima_de_15_mb() {
    let server = MockServer::start().await;
    Mock::given(path("/large"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0; MAX_DOWNLOAD_BYTES + 1]))
        .mount(&server)
        .await;
    let client = ArtworkClient::default();
    assert_eq!(
        client
            .download(&format!("{}/large", server.uri()))
            .await
            .unwrap_err()
            .kind(),
        "artwork_size"
    );
}

#[tokio::test]
async fn download_rejeita_stream_acima_de_15_mb_sem_content_length() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        let mut received = 0;
        while !request[..received].ends_with(b"\r\n\r\n") {
            let count = socket.read(&mut request[received..]).await.unwrap();
            assert!(count > 0, "requisição HTTP incompleta");
            received += count;
        }
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let chunk = vec![0; 64 * 1024];
        for _ in 0..=MAX_DOWNLOAD_BYTES / chunk.len() {
            if socket.write_all(b"10000\r\n").await.is_err()
                || socket.write_all(&chunk).await.is_err()
                || socket.write_all(b"\r\n").await.is_err()
            {
                return;
            }
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });
    let error = ArtworkClient::default()
        .download(&format!("http://{addr}/large"))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), "artwork_size");
    server.await.unwrap();
}
