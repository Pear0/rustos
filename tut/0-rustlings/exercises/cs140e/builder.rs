// FIXME: Make me pass! Diff budget: 30 lines.

#[derive(Default)]
struct Builder {
    string: Option<String>,
    number: Option<usize>,
}

impl Builder {
    // fn string(...
    fn string<T: AsRef<str>>(mut self, s: T) -> Self {
        self.string = Some(String::from(s.as_ref()));
        self
    }

    fn number(mut self, s: i32) -> Self {
        self.number = Some(s as usize);
        self
    }
    // fn number(...
}

impl ToString for Builder {
    fn to_string(&self) -> String {
        let mut b = String::new();
        if let Some(x) = &self.string {
            b.push_str(&x[..]);   
            if let Some(_) = self.number {
                b.push(' ');
            }
        }
        if let Some(x) = self.number {
            let s = format!("{}", x);
            b.push_str(&s[..]);
        }
        b
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

    let c = Builder::default().string("heap!".to_owned()).to_string();

    assert_eq!(c, "heap!");
}
