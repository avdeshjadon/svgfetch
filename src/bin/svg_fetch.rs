//! Alias binary: `svg-fetch` behaves exactly like `svgfetch`.

#[tokio::main]
async fn main() {
    let code = svgfetch::run().await;
    std::process::exit(code);
}
