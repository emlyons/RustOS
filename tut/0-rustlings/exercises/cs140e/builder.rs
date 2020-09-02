// FIXME: Make me pass! Diff budget: 30 lines.

#[derive(Default)]
struct Builder {
    string: Option<String>,
    number: Option<usize>,
}

impl Builder {

    fn string<S: Into<String>>(&mut self, s: S) -> &mut Self {
       self.string = Some(s.into());
       self
    }

    fn get_string(&self) -> String {
       match &self.string {
       	     None => format!(""),
       	     Some(S) => format!("{}", S),
       }
    }

    fn number(&mut self, N: usize) -> &mut Self {
       self.number = Some(N);
       self
    }

    fn get_number(&self) -> usize {
       match &self.number {
       	     None => 0,
       	     Some(N) => *N,
       }
    }
}

impl ToString for Builder {
   fn to_string(&self) -> String {

      let string: String = self.get_string();
      let number: usize = self.get_number();

      match (&*string, number) {
      	    ("", 0) => format!(""),
	    (S, 0) => format!("{}", S),
	    ("", N) => format!("{}", N),
	    (S, N) => format!("{} {}", S, N),
      }
   }
}

// Do not modify this function.
#[test]
fn builder() {
    let empty = Builder::default().to_string();
    assert_eq!(empty, "");

    let just_str = Builder::default().string("hi").to_string();
    assert_eq!(just_str, "hi");

    let just_num = Builder::default().number(254).to_string();
    assert_eq!(just_num, "254");

    let a = Builder::default()
        .string("hello, world!")
        .number(200)
        .to_string();

    assert_eq!(a, "hello, world! 200");

    let b = Builder::default()
        .string("hello, world!")
        .number(200)
        .string("bye now!")
        .to_string();

    assert_eq!(b, "bye now! 200");

    let c = Builder::default()
        .string("heap!".to_owned())
        .to_string();

    assert_eq!(c, "heap!");
}
