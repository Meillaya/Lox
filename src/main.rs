use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::rc::Rc;

mod tokenizer;
mod parser;
mod evaluator;
mod environment;
mod resolver;

use evaluator::{RuntimeError, Interpreter};
use tokenizer::{Tokenizer, TokenType, Token};
use parser::{Parser, print_ast};
use resolver::Resolver;

fn read_and_tokenize(filename: &str) -> Result<Vec<Token>, String> {
    let file_contents = fs::read_to_string(filename).unwrap_or_else(|_| {
        writeln!(io::stderr(), "Failed to read file {}", filename).unwrap();
        process::exit(1);
    });

    if file_contents.is_empty() {
        return Ok(vec![Token {
            token_type: TokenType::EOF,
            lexeme: String::new(),
            literal: None,
            line: 1,
        }]);
    }

    let mut tokenizer = Tokenizer::new(&file_contents);
    let tokens = tokenizer.scan_tokens();

    if tokenizer.has_error {
        return Err("Tokenization error".to_string());
    } else {
        Ok(tokens)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        writeln!(io::stderr(), "Usage: {} tokenize <filename>", args[0]).unwrap();
        return;
    }

    let command = &args[1];
    let filename = &args[2];

    match command.as_str() {
        "tokenize" => {
            let file_contents = fs::read_to_string(filename).unwrap_or_else(|_| {
                writeln!(io::stderr(), "Failed to read file {}", filename).unwrap();
                process::exit(1);
            });

            if !file_contents.is_empty() {
                
                let mut tokenizer = Tokenizer::new(&file_contents);

                let tokens = tokenizer.scan_tokens();
                for token in tokens {

                    if token.token_type != TokenType::WhiteSpace{
                        println!("{} {} {}", token.token_type, token.lexeme, token.literal.as_deref().unwrap_or("null"));
                    }
                }
            if tokenizer.has_error {
                    std::process::exit(65);
                }
            } else {
                println!("EOF  null");
            }
        },
        "parse" => {
            match read_and_tokenize(filename) {
                Ok(tokens) => {
                    let mut parser = Parser::new(tokens);
                    match parser.parse() {
                        Ok(statements) => {
                            if let Some(stmt) = statements.first() {
                                if let parser::Stmt::Expression(expr) = stmt {
                                    println!("{}", print_ast(expr));
                                } else {
                                    println!("First statement is not an expression");
                                }
                            } else {
                                println!("No statements to print");
                            }
                        },
                        Err(error) => {
                            eprintln!("Error: {}", error);
                            process::exit(65);
                        }
                    }
                },
                Err(error) => {
                    eprintln!("Error: {}", error);
                    process::exit(65);
                }
            }
        },
        "evaluate" => {
            match read_and_tokenize(filename) {
                Ok(tokens) => {
                    let mut parser = Parser::new(tokens);
                    match parser.parse() {
                        Ok(statements) => {
                            // Wrap statements in Rc for the interpreter
                            let statements_rc = Rc::new(statements);
                            
                            // Pass Rc to Interpreter::new
                            let mut interpreter = Interpreter::new(Rc::clone(&statements_rc));
                            
                            // Resolve variables (still needs statements slice)
                            let mut resolver = Resolver::new();
                            match resolver.resolve(&*statements_rc) { // Pass slice via deref
                                Ok(_) => {
                                    interpreter.set_locals(resolver.get_locals().clone());
                                    // Also set super expressions
                                    interpreter.set_super_expressions(resolver.get_super_expressions().clone());
                                    
                                    // Call interpret with statements slice
                                    match interpreter.interpret(&*statements_rc, true) {
                                        Ok(_) => {},
                                        Err(runtime_error) => {
                                            match runtime_error {
                                                RuntimeError::Error { message, line } => {
                                                    eprintln!("{} [line {}]", message, line);
                                                    process::exit(70);
                                                },
                                                RuntimeError::Return(_) => {
                                                    // Return statements should be handled within function calls
                                                    process::exit(70);
                                                }
                                            }
                                        }
                                    }
                                },
                                Err(error) => {
                                    eprintln!("Error: {}", error);
                                    process::exit(65);
                                }
                            }
                        },
                        Err(error) => {
                            eprintln!("Error: {}", error);
                            process::exit(65);
                        }
                    }
                },
                Err(error) => {
                    eprintln!("Error: {}", error);
                    process::exit(65);
                }
            }
        },
        "run" => {
            match read_and_tokenize(filename) {
                Ok(tokens) => {
                    let mut parser = Parser::new(tokens);
                    match parser.parse() {
                        Ok(statements) => {
                            // Wrap statements in Rc for the interpreter
                            let statements_rc = Rc::new(statements);

                            // Pass Rc to Interpreter::new
                            let mut interpreter = Interpreter::new(Rc::clone(&statements_rc));

                            // Resolve variables (still needs statements slice)
                            let mut resolver = Resolver::new();
                            match resolver.resolve(&*statements_rc) { // Pass slice via deref
                                Ok(_) => {
                                    interpreter.set_locals(resolver.get_locals().clone());
                                    // Also set super expressions
                                    interpreter.set_super_expressions(resolver.get_super_expressions().clone());
                                    
                                    // Call interpret with statements slice
                                    match interpreter.interpret(&*statements_rc, false) {
                                        Ok(_) => {},
                                        Err(runtime_error) => {
                                            match runtime_error {
                                                RuntimeError::Error { message, line } => {
                                                    eprintln!("{} [line {}]", message, line);
                                                    process::exit(70);
                                                },
                                                RuntimeError::Return(_) => {
                                                    // Return statements should be handled within function calls
                                                    process::exit(70);
                                                }
                                            }
                                        }
                                    }
                                },
                                Err(error) => {
                                    eprintln!("Error: {}", error);
                                    process::exit(65);
                                }
                            }
                        },
                        Err(error) => {
                            eprintln!("Error: {}", error);
                            process::exit(65);
                        }
                    }
                },
                Err(error) => {
                    eprintln!("Error: {}", error);
                    process::exit(65);
                }
            }
        },
        _ => {
            writeln!(io::stderr(), "Unknown command: {}", command).unwrap();
            process::exit(1);
        }
    }
}