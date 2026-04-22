use crate::{
    errors::{FileFormat, FileOperation, IoError, ParseError},
    source::Source,
};
use achitekfile::{
    from_str, ComparisonOperator, Dependency as AchitekDependency, Prompt, PromptType, Value,
};
use indexmap::IndexMap;
use inquire::{
    required, validator::MinLengthValidator, Confirm, Editor, InquireError, MultiSelect, Select,
    Text,
};
use miette::Diagnostic;
use serde::Serialize;
use std::{fs, path::Path};
use thiserror::Error;

const CONFIG_FILE_NAME: &str = "Achitekfile";

#[derive(Debug, Error, Diagnostic)]
pub enum PromptError {
    #[error("I/O error within prompt domain")]
    #[diagnostic(code(achitek::prompt::io))]
    Io(#[from] IoError),

    #[error("Parsing error within prompt domain")]
    #[diagnostic(code(achitek::prompt::parse))]
    Parse(#[from] ParseError),

    #[error("I/O error within prompt domain")]
    #[diagnostic(code(achitek::prompt::prompt))]
    Prompt {
        question: String,
        source: InquireError,
    },
}

/// Represents an answer to a prompt.
#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum Answer {
    String(String),
    // Int(i64),
    // Float(f64),
    Bool(bool),
    Array(Vec<String>),
}

/// Prompts the user with a question based on its configuration, and stores the answer.
fn try_prompt(prompt: &Prompt, answers: &mut IndexMap<String, Answer>) -> Result<(), PromptError> {
    let question = prompt.name.as_str();
    let help = prompt.help.as_deref().unwrap_or("");
    let required_answer = prompt.required.unwrap_or(true);

    match prompt.prompt_type {
        PromptType::String => {
            let mut text = Text::new(question).with_help_message(help);
            if required_answer {
                text = text.with_validator(required!(format!("{} is required", question)));
            }
            if let Some(default) = prompt.default.as_ref().and_then(value_as_text) {
                text = text.with_initial_value(default);
            }
            let answer = text.prompt().map_err(|error| PromptError::Prompt {
                question: question.to_string(),
                source: error,
            })?;

            answers.insert(question.to_string(), Answer::String(answer));
        }
        PromptType::Paragraph => {
            let editor = Editor::new(question)
                .with_formatter(&|submission| {
                    if submission.is_empty() {
                        String::from("<skipped>")
                    } else {
                        submission.into()
                    }
                })
                .with_help_message(help);
            let answer = editor.prompt().map_err(|error| PromptError::Prompt {
                question: question.to_string(),
                source: error,
            })?;

            answers.insert(question.to_string(), Answer::String(answer));
        }
        PromptType::Bool => {
            let mut confirm = Confirm::new(question).with_help_message(help);
            if let Some(Value::Bool(default)) = prompt.default {
                confirm = confirm.with_default(default);
            }
            let answer = confirm.prompt().map_err(|error| PromptError::Prompt {
                question: question.to_string(),
                source: error,
            })?;

            answers.insert(question.to_string(), Answer::Bool(answer));
        }
        PromptType::Select => {
            let choices = prompt
                .choices
                .iter()
                .map(value_to_choice)
                .collect::<Vec<_>>();
            if !choices.is_empty() {
                let answer = Select::new(question, choices)
                    .with_help_message(help)
                    .prompt()
                    .map_err(|error| PromptError::Prompt {
                        question: question.to_string(),
                        source: error,
                    })?;

                answers.insert(question.to_string(), Answer::String(answer));
            }
        }
        PromptType::MultiSelect => {
            let choices = prompt
                .choices
                .iter()
                .map(value_to_choice)
                .collect::<Vec<_>>();
            if !choices.is_empty() {
                let min_selections = prompt.validation.min_selections.unwrap_or(1) as usize;
                let answer = MultiSelect::new(question, choices)
                    .with_help_message(help)
                    .with_validator(MinLengthValidator::new(min_selections))
                    .prompt()
                    .map_err(|error| PromptError::Prompt {
                        question: question.to_string(),
                        source: error,
                    })?;

                answers.insert(question.to_string(), Answer::Array(answer));
            }
        }
    }

    Ok(())
}

fn value_as_text(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) | Value::Identifier(value) => Some(value.as_str()),
        _ => None,
    }
}

fn value_to_choice(value: &Value) -> String {
    match value {
        Value::String(value) | Value::Identifier(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Array(_) => String::new(),
    }
}

fn should_prompt(prompt: &Prompt, answers: &IndexMap<String, Answer>) -> bool {
    prompt
        .depends_on
        .as_ref()
        .is_none_or(|dependency| dependency_is_satisfied(dependency, answers))
}

fn dependency_is_satisfied(
    dependency: &AchitekDependency,
    answers: &IndexMap<String, Answer>,
) -> bool {
    match dependency {
        AchitekDependency::Reference(name) => answers.get(name).is_some_and(answer_is_truthy),
        AchitekDependency::Comparison {
            left,
            operator,
            right,
        } => {
            let Some(answer) = answers.get(left) else {
                return false;
            };
            let matches = answer_matches_value(answer, right);
            match operator {
                ComparisonOperator::Equal => matches,
                ComparisonOperator::NotEqual => !matches,
            }
        }
        AchitekDependency::Contains { receiver, argument } => answers
            .get(receiver)
            .is_some_and(|answer| answer_contains_value(answer, argument)),
        AchitekDependency::All(dependencies) => dependencies
            .iter()
            .all(|dependency| dependency_is_satisfied(dependency, answers)),
        AchitekDependency::Any(dependencies) => dependencies
            .iter()
            .any(|dependency| dependency_is_satisfied(dependency, answers)),
    }
}

fn answer_is_truthy(answer: &Answer) -> bool {
    match answer {
        Answer::String(value) => !value.is_empty(),
        Answer::Bool(value) => *value,
        Answer::Array(value) => !value.is_empty(),
    }
}

fn answer_matches_value(answer: &Answer, value: &Value) -> bool {
    match (answer, value) {
        (Answer::String(answer), Value::String(value) | Value::Identifier(value)) => {
            answer == value
        }
        (Answer::String(answer), Value::Integer(value)) => answer == &value.to_string(),
        (Answer::Bool(answer), Value::Bool(value)) => answer == value,
        (Answer::Array(answer), Value::Array(values)) => {
            let expected = values.iter().map(value_to_choice).collect::<Vec<_>>();
            answer == &expected
        }
        (Answer::Array(answer), value) => {
            answer_contains_value(&Answer::Array(answer.clone()), value)
        }
        _ => false,
    }
}

fn answer_contains_value(answer: &Answer, value: &Value) -> bool {
    match answer {
        Answer::Array(values) => {
            let expected = value_to_choice(value);
            values.iter().any(|answer| answer == &expected)
        }
        Answer::String(answer) => value_as_text(value).is_some_and(|value| answer.contains(value)),
        Answer::Bool(answer) => matches!(value, Value::Bool(value) if answer == value),
    }
}

/// Processes the questions file and gathers user answers.
///
/// This function:
/// - reads an Achitekfile
/// - asks prompts in dependency order
/// - evaluates each prompt dependency against answers gathered so far.
pub fn get_answers(template_path: &Path) -> Result<IndexMap<String, Answer>, PromptError> {
    let config_path = template_path.join(CONFIG_FILE_NAME);
    let content = fs::read_to_string(config_path.clone())
        .map_err(|err| IoError::new(FileOperation::Read, config_path.clone(), err))?;
    let ast = from_str(&content)
        .map_err(|err| ParseError::new(FileFormat::Achitekfile, config_path.clone(), err))?;
    let prompts = ast
        .ordered_prompts()
        .map_err(|err| ParseError::new(FileFormat::Achitekfile, config_path, err))?;
    let mut answers = IndexMap::new();

    for prompt in prompts {
        if should_prompt(&prompt, &answers) {
            try_prompt(&prompt, &mut answers)?;
        }
    }

    Ok(answers)
}

pub fn get_project(config: Source) -> Result<String, PromptError> {
    let choices = config.blueprints.keys().collect();

    let question = String::from("Select template:");

    let answer = Select::new(&question, choices)
        .prompt()
        .map_err(|error| PromptError::Prompt {
            question: question.to_string(),
            source: error,
        })?;

    Ok(answer.to_owned())
}

pub fn get_destination() -> Result<String, PromptError> {
    let question = String::from("Destination");

    let answer = Text::new(&question)
        .prompt()
        .map_err(|error| PromptError::Prompt {
            question: question.to_string(),
            source: error,
        })?;

    Ok(answer.to_owned())
}

pub fn apply_changes() -> Result<bool, PromptError> {
    let question = String::from("Apply changes?");

    let answer = Confirm::new(&question)
        .prompt()
        .map_err(|error| PromptError::Prompt {
            question: question.to_string(),
            source: error,
        })?;

    Ok(answer.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use achitekfile::Validation;

    fn prompt_with_dependency(depends_on: Option<AchitekDependency>) -> Prompt {
        Prompt {
            name: "dependent".to_string(),
            prompt_type: PromptType::String,
            help: None,
            choices: Vec::new(),
            default: None,
            required: None,
            depends_on,
            validation: Validation::default(),
        }
    }

    #[test]
    fn prompts_without_dependencies_are_asked() {
        let prompt = prompt_with_dependency(None);

        assert!(should_prompt(&prompt, &IndexMap::new()));
    }

    #[test]
    fn evaluates_dependency_expressions_against_existing_answers() {
        let mut answers = IndexMap::new();
        answers.insert(
            "database".to_string(),
            Answer::String("postgres".to_string()),
        );
        answers.insert(
            "features".to_string(),
            Answer::Array(vec!["auth".to_string(), "metrics".to_string()]),
        );

        let prompt = prompt_with_dependency(Some(AchitekDependency::All(vec![
            AchitekDependency::Comparison {
                left: "database".to_string(),
                operator: ComparisonOperator::Equal,
                right: Value::String("postgres".to_string()),
            },
            AchitekDependency::Contains {
                receiver: "features".to_string(),
                argument: Value::String("auth".to_string()),
            },
        ])));

        assert!(should_prompt(&prompt, &answers));
    }

    #[test]
    fn skips_prompts_when_dependencies_are_not_satisfied() {
        let mut answers = IndexMap::new();
        answers.insert("database".to_string(), Answer::String("sqlite".to_string()));

        let prompt = prompt_with_dependency(Some(AchitekDependency::Comparison {
            left: "database".to_string(),
            operator: ComparisonOperator::NotEqual,
            right: Value::String("sqlite".to_string()),
        }));

        assert!(!should_prompt(&prompt, &answers));
    }
}
