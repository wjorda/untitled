use common::{Command, Config, Response};

fn main() {
    println!("{}", serde_json::to_string(&Command::Start).unwrap());

    let test = Command::SetConfig {
        config: Config { seed: 0 },
    };

    println!("{}", serde_json::to_string(&test).unwrap());

    println!("{}", serde_json::to_string(&Response::Ok).unwrap());

    let testresp = Response::RandomInt { value: 12345 };

    println!("{}", serde_json::to_string(&testresp).unwrap());
}
