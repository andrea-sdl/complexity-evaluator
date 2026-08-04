fn main() {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("error: cannot read working directory: {error}");
            std::process::exit(2);
        }
    };
    let output = complexity::run(std::env::args_os().skip(1), &cwd);
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    if output.exit_code != 0 {
        std::process::exit(output.exit_code.into());
    }
}
