use anyhow::Result;

mod app;
mod components;
mod theme;
mod views;

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}
