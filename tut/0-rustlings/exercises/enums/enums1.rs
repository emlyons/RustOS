// enums1.rs
// Make me compile! Execute `rustlings hint enums1` for hints!

#[derive(Debug)]
enum Message {
    Quit(bool),
    Echo,
    Move {x: i32 ,y: i32},
    ChangeColor(String),
}

fn main() {
    println!("{:?}", Message::Quit(true));
    println!("{:?}", Message::Echo);
    println!("{:?}", Message::Move{x: 3, y: 5});
    println!("{:?}", Message::ChangeColor(String::from("blue")));
}
