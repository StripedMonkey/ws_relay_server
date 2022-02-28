use rand::{self, prelude::SliceRandom, rngs::OsRng};

pub(crate) struct WordChooser {}

impl WordChooser {
    pub fn new(first: Vec<&str>, second: Vec<&str>, third: Vec<&str>) -> WordChooser {
        WordChooser {}
    }

    pub fn generate_room_name(&mut self) -> String {
        let mut s = String::new();
        s.push_str("Wow");
        s
    }
}

pub(crate) fn generate_room_name() -> String {
    let data: Vec<String> = include_str!("WordList.txt")
        .split("\n")
        .map(|s| s.to_string())
        .collect();
    data.choose(&mut OsRng).unwrap().clone()
}

#[cfg(test)]
mod test {
    use crate::room::RoomRequest;

    #[test]
    fn room_format() {
        let new_room = RoomRequest::NewRoom;

        println!("New room: {}", serde_json::to_string(&new_room).unwrap());

        let existing_room = RoomRequest::JoinRoom("Amazing".to_string());
        print!(
            "Existing room: {}",
            serde_json::to_string(&existing_room).unwrap()
        );
    }
}
