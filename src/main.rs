use clap::Parser;
use herbivore::websocket_client::{NodeType, WebSocketClient};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    user_id: String,

    #[arg(short, long, default_value = "1.25x", value_parser = ["1x", "2x", "1.25x"])]
    node_type: String,
}

#[tokio::main]
async fn main() {

    env_logger::init();

    let args = Args::parse();
    let mut websocket_client = WebSocketClient::new(args.user_id, NodeType::from_str(&args.node_type));
    websocket_client.start().await;
}
