//! Legacy alias binary: `get-svg` behaves exactly like `svgfetch`.

#[tokio::main]
async fn main() {
    let code = svgfetch::run().await;
    std::process::exit(code);
}
