use clap::Parser;
use image_processor::process_image;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the input image file
    #[arg(short, long)]
    input: String,

    /// Path to the output image file
    #[arg(short, long)]
    output: String,

    /// Plugin path to use for processing
    #[arg(short, long)]
    dir: String,

    /// Plugin path to use for processing
    #[arg(short, long)]
    name: String,

    /// Plugin arguments as JSON string
    #[arg(short, long)]
    params_path: String,
}

fn main() {
    let args = Args::parse();
    let input = Path::new(args.input.as_str());
    let output = Path::new(args.output.as_str());
    let plugin_dir = Path::new(args.dir.as_str());
    let plugin_name = args.name;
    let params_path = Path::new(args.params_path.as_str());

    match process_image(input, output, plugin_dir, &plugin_name, params_path) {
        Ok(_) => {
            println!("Process image successfull");
        }
        Err(e) => {
            println!("Can't process image: {e}");
        }
    }
}
