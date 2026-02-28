pub mod cli {
    use clap::Parser;

    #[derive(Parser, Debug)]
    #[command(name = "matrix-bridge-dingtalk")]
    #[command(about = "A Matrix <-> DingTalk bridge", long_about = None)]
    pub struct Args {
        #[arg(short, long, env = "CONFIG_PATH", default_value = "config.yaml")]
        pub config: String,
    }
}
