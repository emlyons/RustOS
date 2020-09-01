// FIXME: Make me compile! Diff budget: 1 line.

// Do not change this module.
mod a {
    pub trait MyTrait {
        fn foo(&self) {}
    }

    pub struct MyType;

    impl MyTrait for MyType {}
}

// Do not modify this function.
fn main() {
    use a::MyTrait;
    
    let x = a::MyType;
    x.foo();
}
