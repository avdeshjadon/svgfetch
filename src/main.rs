//! svgfetch binary entry point.

#[tokio::main]
async fn main() {
    let code = svgfetch::run().await;
    std::process::exit(code);
}
