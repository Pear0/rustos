// This does practically the same thing that TryFrom<&str> does.
// Additionally, upon implementing FromStr, you can use the `parse` method
// on strings to generate an object of the implementor type.
// You can read more about it at https://doc.rust-lang.org/std/str/trait.FromStr.html
use std::str::FromStr;

#[derive(Debug)]
struct Person {
    name: String,
    age: usize,
}

// Steps:
// 1. Split the given string on the commas present in it
// 2. Extract the first element from the split operation and use it as the name
// 3. Extract the other element from the split operation and parse it into a `usize` as the age
// If something goes wrong, for instance there is no comma in the provided string or
// parsing the age fails, then return Err of String
// Otherwise, return Ok result of a Person object
impl FromStr for Person {
    type Err = String;
    fn from_str(s: &str) -> Result<Person, Self::Err> {


        let split = s.split(",").collect::<Vec<&str>>();
        if split.len() != 2 {
            return Err(split[0].to_string());
        }

        match split[1].parse::<usize>() {
            Ok(num) => Ok(Person{name: split[0].to_string(), age: num}),
            default => Err(split[0].to_string()),
        }

    }
}

fn main() {
    let p = "Mark,20".parse::<Person>().unwrap();
    println!("{:?}", p);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert!("".parse::<Person>().is_err());
    }
    #[test]
    fn good_input() {
        assert!("John,32".parse::<Person>().is_ok());
    }
    #[test]
    #[should_panic]
    fn missing_age() {
        "John".parse::<Person>().unwrap();
    }
}
