use clap::Parser;
use figlet_rs::FIGfont;
use herbivore::websocket_client::{NodeType, WebSocketClient};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, required = true)]
    user_id: String,

    #[arg(long, default_value = "1.25x", value_parser = ["1x", "2x", "1.25x"])]
    node_type: String,

    #[arg(long, default_value = "info", value_parser = ["info", "debug", "error", "warn"])]
    log_level: String,

    #[arg(long)]
    log_file: Option<String>,
}

#[tokio::main]
async fn main() {


    let args = Args::parse();

    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("Herbivore");
    if let Some(figure) = figure {
        println!("\x1b[32m{}\x1b[0m", figure);
    }

    match args.log_file {
        Some(log_file) => {
            // Ensure directory exists
            if let Some(parent) = std::path::Path::new(&log_file).parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).expect("Failed to create log directory");
                }
            }
            
            let log_file = std::fs::File::create(&log_file)
                .expect("Failed to create log file");
                
            env_logger::Builder::new()
                .filter_level(args.log_level.parse().unwrap())
                .target(env_logger::Target::Pipe(Box::new(log_file)))
                .init();
        }
        None => {
            env_logger::Builder::new()
                .filter_level(args.log_level.parse().unwrap())
                .init();
        }
    }

    let mut websocket_client = WebSocketClient::new(args.user_id, NodeType::from_str(&args.node_type));
    websocket_client.start().await;
}
