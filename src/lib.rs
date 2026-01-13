use std::collections::HashMap;

struct Environment {}

struct Environments {
    default: String,
    environments: HashMap<String, Environment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
