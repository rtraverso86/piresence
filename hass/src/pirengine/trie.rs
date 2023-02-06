use std::collections::HashMap;

pub trait TrieLookup<T> {
    fn insert(&mut self, key: &str, value: T);
    fn search(&self, key: &str) -> Option<&T>;
}


/* HashedTrie<T> */

struct HashedTrieNode<T> {
    children: HashMap<char, HashedTrieNode<T>>,
    value: Option<T>,
}

impl<T> HashedTrieNode<T> {
    fn new() -> HashedTrieNode<T> {
        HashedTrieNode {
            children: HashMap::new(),
            value: None,
        }
    }
}

pub struct HashedTrie<T> {
    root: HashedTrieNode<T>,
}

impl<T> HashedTrie<T> {
    pub fn new() -> HashedTrie<T> {
        HashedTrie { root: HashedTrieNode::new() }
    }
}

impl<T> TrieLookup<T> for HashedTrie<T> {
    fn insert(&mut self, key: &str, value: T) {
        let mut current_node = &mut self.root;
        for c in key.chars() {
            current_node = current_node.children.entry(c).or_insert(HashedTrieNode::new());
        }
        current_node.value = Some(value);
    }

    fn search(&self, key: &str) -> Option<&T> {
        let mut current_node = &self.root;
        for c in key.chars() {
            current_node = current_node.children.get(&c)?;
        }
        current_node.value.as_ref()
    }
}


/* HashMapTrie<T> */

pub struct HashMapTrie<T>(HashMap<String, T>);

impl<T> HashMapTrie<T> {
    pub fn new() -> HashMapTrie<T> {
        HashMapTrie ( HashMap::new() )
    }
}

impl<T> TrieLookup<T> for HashMapTrie<T> {
    fn insert(&mut self, key: &str, value: T) {
        self.0.insert(key.to_owned(), value);
    }

    fn search(&self, key: &str) -> Option<&T> {
        self.0.get(key)
    }
}


/* Trie<T> */

struct TrieNode<T> {
    children: Vec<(char, TrieNode<T>)>,
    value: Option<T>,
}

impl<T> TrieNode<T> {
    fn new(value: Option<T>) -> Self {
        Self { children: Vec::new(), value }
    }
}

pub struct Trie<T> {
    root: TrieNode<T>,
}

impl<T> Trie<T> {
    pub fn new() -> Self {
        Self { root: TrieNode::new(None) }
    }
}

impl<T> TrieLookup<T> for Trie<T> {
    fn insert(&mut self, key: &str, value: T) {
        let mut node = &mut self.root;
        for c in key.chars() {
            let idx = node.children
                .binary_search_by(|(k, _)| k.cmp(&c) )
                .unwrap_or_else(|i| {
                    node.children.insert(i, (c, TrieNode::new(None)));
                    i
                });
            node = &mut node.children[idx].1
        }
        node.value = Some(value);
    }

    fn search(&self, key: &str) -> Option<&T> {
        let mut node = &self.root;
        for c in key.chars() {
            match node.children.binary_search_by(|(k, _)| k.cmp(&c)) {
                Ok(i) => {
                    node = &node.children[i].1;
                },
                _ => {
                    return None;
                }
            };
        }
        node.value.as_ref()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn insert_and_search(trie: &mut dyn TrieLookup<i32>) {
        trie.insert("hello", 42);
        trie.insert("hey", 13);
        trie.insert("another", 54);

        assert_eq!(trie.search("hello"), Some(&42));
        assert_eq!(trie.search("hey"), Some(&13));
        assert_eq!(trie.search("hi"), None);
        assert_eq!(trie.search("another"), Some(&54));
    }

    #[test]
    fn hashedtrie_insert_and_search() {
        insert_and_search(&mut HashedTrie::new());
    }

    #[test]
    fn hashmap_insert_and_search() {
        insert_and_search(&mut HashMapTrie::new());
    }

    #[test]
    fn basic_insert_and_search() {
        insert_and_search(&mut Trie::new());
    }
}
