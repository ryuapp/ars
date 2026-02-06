use ars::Url;

fn main() {
    // Parse a simple URL
    let url = Url::parse("https://example.com:8080/path?query=value#hash", None)
        .expect("Failed to parse URL");

    println!("URL: {}", url.href()); // https://example.com:8080/path?query=value#hash
    println!("Protocol: {}", url.protocol()); // https:
    println!("Host: {}", url.host()); // example.com:8080
    println!("Port: {}", url.port()); // 8080
    println!("Pathname: {}", url.pathname()); // /path
    println!("Search: {}", url.search()); // ?query=value
    println!("Hash: {}", url.hash()); // #hash
}
