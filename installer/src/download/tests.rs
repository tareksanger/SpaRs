use super::*;
#[test]
fn download_stream_checks_digest_and_propagates_read_errors() {
    let path = std::env::temp_dir().join(format!("spars-download-test-{}", std::process::id()));
    assert!(copy_checked(
        &b"abc"[..],
        File::create(&path).unwrap(),
        &Digest::of(b"abc")
    )
    .is_ok());
    assert!(copy_checked(
        &b"ab"[..],
        File::create(&path).unwrap(),
        &Digest::of(b"abc")
    )
    .is_err());
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("interrupted network"))
        }
    }
    assert!(copy_checked(Broken, File::create(&path).unwrap(), &Digest::of(b"abc")).is_err());
    std::fs::remove_file(path).unwrap();
}

fn server(responses: Vec<&'static [u8]>) -> (String, std::thread::JoinHandle<()>) {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let thread = std::thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(response);
        }
    });
    (format!("http://{address}/model"), thread)
}
#[test]
fn retries_status_and_rejects_truncation() {
    let dir = std::env::temp_dir().join(format!("spars-http-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("archive");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .into();
    let (url, worker) = server(vec![
        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc",
    ]);
    fetch(&agent, &url, &path, &Digest::of(b"abc")).unwrap();
    worker.join().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"abc");
    let (url, worker) =
        server(vec![b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nab"; 3]);
    assert!(fetch(&agent, &url, &path, &Digest::of(b"abc")).is_err());
    worker.join().unwrap();
    assert!(!path.exists());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn https_only_prevents_downgrade_and_timeouts_are_bounded() {
    let dir = std::env::temp_dir().join(format!("spars-timeout-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("archive");
    let secure: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .build()
        .into();
    assert!(fetch(
        &secure,
        "http://127.0.0.1:1/model",
        &path,
        &Digest::of(b"abc")
    )
    .is_err());
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/model", listener.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        for _ in 0..3 {
            let (stream, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(70));
            drop(stream);
        }
    });
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_millis(30)))
        .build()
        .into();
    assert!(fetch(&agent, &url, &path, &Digest::of(b"abc")).is_err());
    worker.join().unwrap();
    assert!(!path.exists());
    std::fs::remove_dir_all(dir).unwrap();
}
