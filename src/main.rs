use std::io;

fn main() {
    loop {
        let mut input = String::new();

        io::stdin().read_line(&mut input)
            .ok()
            .expect("failed to read line");

        println!("No, you're a {}", input);
    }
}
