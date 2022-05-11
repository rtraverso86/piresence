use std::io::{self, BufRead};

/// Extends `BufReader<T>` with methods useful for reading chunks of data
/// that represent individual, complete yaml documents.
trait BufReaderYamlExt {
    /// Reads entire lines, stopping (and consuming) at the next document
    /// separator `---` and returning the accumulated string so far.
    fn read_next_yaml(&mut self) -> io::Result<Option<String>>;
}

impl<T: io::Read> BufReaderYamlExt for io::BufReader<T> {
    fn read_next_yaml(&mut self) -> io::Result<Option<String>> {
        const sep : &str = "\n---\n";
        let mut doc = String::with_capacity(1100);
        while self.read_line(&mut doc)? > 0 {
            if doc.len() > sep.len() && doc.ends_with(sep) {
                doc.truncate(doc.len() - sep.len());
                break;
            }
        }
        if !doc.is_empty() {
            doc.shrink_to_fit();
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }
}


/***** TESTS *****************************************************************/

#[cfg(test)]
mod test {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq, Eq)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum Message {
        Event { id: i32 },
        Trigger { id: i32}, 
    }

    fn check(yaml: &str, expected: Vec<Message>) {
        let mut r = io::BufReader::new(yaml.as_bytes());
        let mut i = 0;
        while let Some(next) = r.read_next_yaml().unwrap() {
            let d: Message = serde_yaml::from_str(&next).unwrap();
            assert_eq!(expected[i], d);
            i += 1;
        }
    }


    #[test]
    fn single_document() {
        check("---\ntype: event\nid: 1",
            vec![
                Message::Event { id: 1 },
            ]);
    }

    #[test]
    fn single_document_newlines() {
        check("---\n\ntype: event\n\nid: 1\n\n",
            vec![
                Message::Event { id: 1 },
            ]);
    }

    #[test]
    fn no_initial_marker() {
        check("type: event\nid: 1",
            vec![
                Message::Event { id: 1 },
            ]);
    }

    #[test]
    fn multiple_documents() {
        check("---\ntype: event\nid: 1\n---\n\ntype: trigger\nid: 2",
            vec![
                Message::Event { id: 1 },
                Message::Trigger { id: 2 },
            ]);
    }

    #[test]
    fn multiple_documents_newlines() {
        check("\n---\ntype: event\nid: 1\n\n---\n\ntype: trigger\n\nid: 2\n\n---\n\ntype: event\nid: 3",
            vec![
                Message::Event { id: 1 },
                Message::Trigger { id: 2 },
                Message::Event { id: 3 },
            ]);
    }

}