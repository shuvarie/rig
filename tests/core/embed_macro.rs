//! Implementing `Embed` by hand.
//!
//! The `#[derive(Embed)]` macro is gone; these are the impls it generated, so
//! each case still pins what `to_texts` produces.

use rig::{
    Embed,
    embeddings::{self, TextEmbedder, embed::EmbedError},
};
use serde::Serialize;

#[test]
fn custom_field_embeds_through_a_function() {
    struct WordDefinition {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        word: String,
        definition: Definition,
    }

    #[derive(Serialize, Clone)]
    struct Definition {
        word: String,
        link: String,
        speech: String,
    }

    impl Embed for WordDefinition {
        fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
            custom_embedding_function(embedder, self.definition.clone())
        }
    }

    fn custom_embedding_function(
        embedder: &mut TextEmbedder,
        definition: Definition,
    ) -> Result<(), EmbedError> {
        embedder.embed(serde_json::to_string(&definition).map_err(EmbedError::new)?);

        Ok(())
    }

    let definition = WordDefinition {
        id: "doc1".to_string(),
        word: "house".to_string(),
        definition: Definition {
            speech: "noun".to_string(),
            word: "a building in which people live; residence for human beings.".to_string(),
            link: "https://www.dictionary.com/browse/house".to_string(),
        },
    };

    assert_eq!(
        embeddings::to_texts(definition).unwrap(),
        vec!["{\"word\":\"a building in which people live; residence for human beings.\",\"link\":\"https://www.dictionary.com/browse/house\",\"speech\":\"noun\"}".to_string()]
    );
}

#[test]
fn custom_and_basic_fields_keep_their_order() {
    struct WordDefinition {
        #[allow(dead_code)]
        id: String,
        word: String,
        definition: Definition,
    }

    #[derive(Serialize, Clone)]
    struct Definition {
        word: String,
        link: String,
        speech: String,
    }

    impl Embed for WordDefinition {
        fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
            Embed::embed(&self.word, embedder)?;
            custom_embedding_function(embedder, self.definition.clone())
        }
    }

    fn custom_embedding_function(
        embedder: &mut TextEmbedder,
        definition: Definition,
    ) -> Result<(), EmbedError> {
        embedder.embed(serde_json::to_string(&definition).map_err(EmbedError::new)?);

        Ok(())
    }

    let definition = WordDefinition {
        id: "doc1".to_string(),
        word: "house".to_string(),
        definition: Definition {
            speech: "noun".to_string(),
            word: "a building in which people live; residence for human beings.".to_string(),
            link: "https://www.dictionary.com/browse/house".to_string(),
        },
    };

    let texts = embeddings::to_texts(definition).unwrap();

    assert_eq!(
        texts,
        vec![
            "house".to_string(),
            "{\"word\":\"a building in which people live; residence for human beings.\",\"link\":\"https://www.dictionary.com/browse/house\",\"speech\":\"noun\"}".to_string()
        ]
    );
}

#[test]
fn a_numeric_vector_embeds_one_text_per_element() {
    struct Company {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        company: String,
        employee_ages: Vec<i32>,
    }

    impl Embed for Company {
        fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
            Embed::embed(&self.employee_ages, embedder)
        }
    }

    let company = Company {
        id: "doc1".to_string(),
        company: "Google".to_string(),
        employee_ages: vec![25, 30, 35, 40],
    };

    assert_eq!(
        embeddings::to_texts(company).unwrap(),
        vec![
            "25".to_string(),
            "30".to_string(),
            "35".to_string(),
            "40".to_string()
        ]
    );
}

#[test]
fn a_string_vector_embeds_its_elements() {
    struct Company {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        company: String,
        employee_names: Vec<String>,
    }

    impl Embed for Company {
        fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
            Embed::embed(&self.employee_names, embedder)
        }
    }

    let company = Company {
        id: "doc1".to_string(),
        company: "Google".to_string(),
        employee_names: vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "David".to_string(),
        ],
    };

    assert_eq!(
        embeddings::to_texts(company).unwrap(),
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "David".to_string()
        ]
    );
}

#[test]
fn several_fields_embed_in_declaration_order() {
    struct Company {
        #[allow(dead_code)]
        id: String,
        company: String,
        employee_ages: Vec<i32>,
    }

    impl Embed for Company {
        fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
            Embed::embed(&self.company, embedder)?;
            Embed::embed(&self.employee_ages, embedder)
        }
    }

    let company = Company {
        id: "doc1".to_string(),
        company: "Google".to_string(),
        employee_ages: vec![25, 30, 35, 40],
    };

    assert_eq!(
        embeddings::to_texts(company).unwrap(),
        vec![
            "Google".to_string(),
            "25".to_string(),
            "30".to_string(),
            "35".to_string(),
            "40".to_string()
        ]
    );
}
