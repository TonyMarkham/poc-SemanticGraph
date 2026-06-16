use semantic_graph_cli::run_from_env;

fn main() {
    match run_from_env() {
        Ok(output) => {
            for line in output.lines() {
                println!("{line}");
            }
        }
        Err(error) => {
            eprintln!("{}", error.user_message());
            std::process::exit(error.exit_code());
        }
    }
}
