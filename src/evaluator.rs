use crate::parser::{Expr, LiteralValue, Stmt};
use crate::tokenizer::{Token, TokenType};
use std::fmt;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;


#[derive(Debug, PartialEq)]
pub struct Environment {
    values: HashMap<String, Value>,
    enclosing: Option<Rc<RefCell<Environment>>>,
}



#[derive(Debug)]
pub enum RuntimeError {
    Error { message: String, line: usize },
    Return(Value),
}


impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
            enclosing: None,
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name_token: &Token) -> Result<Value, RuntimeError> {
        if let Some(value) = self.values.get(&name_token.lexeme) {
            Ok(value.clone())
        } else if let Some(ref enclosing) = self.enclosing {
            enclosing.borrow().get(name_token)
        } else {
            Err(RuntimeError::new(
                format!("Undefined variable '{}'", name_token.lexeme),
                name_token.line,
            ))
        }
    }

    pub fn assign(&mut self, name_token: &Token, value: Value) -> Result<(), RuntimeError> {
        if self.values.contains_key(&name_token.lexeme) {
            self.values.insert(name_token.lexeme.clone(), value);
            Ok(())
        } else if let Some(ref enclosing) = self.enclosing {
            enclosing.borrow_mut().assign(name_token, value)
        } else {
            Err(RuntimeError::new(
                format!("Undefined variable '{}'", name_token.lexeme),
                name_token.line,
            ))
        }
    }

    pub fn new_with_enclosing(enclosing: Rc<RefCell<Environment>>) -> Self {
        Environment {
            values: HashMap::new(),
            enclosing: Some(enclosing),
        }
    }

    pub fn define_natives(&mut self) {
        self.define("clock".to_string(), Value::NativeFunction(|| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            Value::Number(now.as_secs_f64())
        }));
    }
}


impl RuntimeError {
    pub fn new(message: String, line: usize) -> Self {
        RuntimeError::Error { message, line }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
    NativeFunction(fn() -> Value),
    Function(String, Vec<Token>, Vec<Stmt>, Rc<RefCell<Environment>>),
}


impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::NativeFunction(_) => write!(f, "<native fn>"),
            Value::Function(name, _, _, _) => write!(f, "<fn {}>", name),
        }
    }
}

fn is_number(value: &Value) -> bool {
    matches!(value, Value::Number(_))
}

fn get_number(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(RuntimeError::new("Operand must be a number.".to_string(), 0)),
    }
}

fn is_string(value: &Value) -> bool {
    matches!(value, Value::String(_))
}

pub fn evaluate(expr: &Expr, env: Rc<RefCell<Environment>>) -> Result<Value, RuntimeError> {
    match expr {
        Expr::Literal(literal) => Ok(match literal {
            LiteralValue::Boolean(value) => Value::Boolean(*value),
            LiteralValue::Number(value) => Value::Number(*value),
            LiteralValue::String(value) => Value::String(value.clone()),
            LiteralValue::Nil => Value::Nil,
        }),
        Expr::Grouping(expr) => evaluate(expr, Rc::clone(&env)),
        Expr::Unary(operator, expr) => {
            let right = evaluate(expr, Rc::clone(&env))?;
            match operator.token_type {
                TokenType::Minus => {
                    if let Value::Number(n) = right {
                        Ok(Value::Number(-n))
                    } else {
                        Err(RuntimeError::new("Operand must be a number.".to_string(), operator.line))
                    }
                },
                TokenType::Bang => Ok(Value::Boolean(!is_truthy(&right))),
                _ => Ok(Value::String("Unimplemented".to_string())),
            }
        },
        Expr::Binary(left, operator, right) => {
            let left = evaluate(left, Rc::clone(&env))?;
            let right = evaluate(right, Rc::clone(&env))?;
            match operator.token_type {
                TokenType::Plus => {
                    if is_number(&left) && is_number(&right) {
                        Ok(Value::Number(get_number(&left)? + get_number(&right)?))
                    } else if is_string(&left) && is_string(&right) {
                        match (&left, &right) {
                            (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
                            _ => unreachable!(),
                        }
                    } else {
                        Err(RuntimeError::new("Operands must be two numbers or two strings.".to_string(), operator.line))
                    }
                },
                TokenType::Minus => {
                    if is_number(&left) && is_number(&right) {
                        Ok(Value::Number(get_number(&left)? - get_number(&right)?))
                    } else {
                        Err(RuntimeError::new("Operands must be numbers.".to_string(), operator.line))
                    }
                },
                TokenType::Star => {
                    if is_number(&left) && is_number(&right) {
                        Ok(Value::Number(get_number(&left)? * get_number(&right)?))
                    } else {
                        Err(RuntimeError::new("Operands must be numbers.".to_string(), operator.line))
                    }
                },
                TokenType::Slash => {
                    if is_number(&left) && is_number(&right) {
                        let right_num = get_number(&right)?;
                        if right_num == 0.0 {
                            Err(RuntimeError::new("Division by zero.".to_string(), operator.line))
                        } else {
                            Ok(Value::Number(get_number(&left)? / right_num))
                        }
                    } else {
                        Err(RuntimeError::new("Operands must be numbers.".to_string(), operator.line))
                    }
                },
                TokenType::Greater => compare_values(&left, &right, |a, b| a > b),
                TokenType::GreaterEqual => compare_values(&left, &right, |a, b| a >= b),
                TokenType::Less => compare_values(&left, &right, |a, b| a < b),
                TokenType::LessEqual => compare_values(&left, &right, |a, b| a <= b),
                TokenType::EqualEqual => {
                    let result = compare_equality(&left, &right)?;
                    Ok(Value::Boolean(result))
                },
                TokenType::BangEqual => {
                    let result = compare_equality(&left, &right)?;
                    Ok(Value::Boolean(!result))
                },
                _ => Ok(Value::String("Unimplemented".to_string())),
            }
        },
        Expr::Variable(name) => {
            env.borrow().get(name).map_err(|err| match err {
                RuntimeError::Error { message, line: _ } => RuntimeError::Error {
                    message: message,
                    line: name.line,
                },
                RuntimeError::Return(value) => RuntimeError::Return(value),
            })
        },
        Expr::Assign(name, value_expr) => {
            let value = evaluate(value_expr, Rc::clone(&env))?;
            env.borrow_mut().assign(name, value.clone())?;
            Ok(value)
        },
        Expr::Logical(left, operator, right) => {
            let left_val = evaluate(left, Rc::clone(&env))?;
            
            if operator.token_type == TokenType::Or {
                if is_truthy(&left_val) {
                    return Ok(left_val);
                }
            } else if operator.token_type == TokenType::And {
                if !is_truthy(&left_val) {
                    return Ok(left_val);
                }
            }
            
            evaluate(right, Rc::clone(&env))
        },
        Expr::Call(callee, paren, arguments) => {
            let callee_val = evaluate(callee, Rc::clone(&env))?;
            
            match callee_val {
                Value::NativeFunction(func) => {
                    if !arguments.is_empty() {
                        return Err(RuntimeError::new(
                            "Native function expects 0 arguments.".to_string(),
                            paren.line,
                        ));
                    }
                    Ok(func())
                }
                Value::Function(_, params, body, closure) => {
                    if arguments.len() != params.len() {
                        return Err(RuntimeError::Error {
                            message: format!("Expected {} arguments but got {}.", 
                                params.len(), arguments.len()),
                            line: paren.line,
                        });
                    }
                    
                    let function_env = Rc::new(RefCell::new(Environment::new_with_enclosing(closure)));
                    
                    for (param, arg) in params.iter().zip(arguments) {
                        let value = evaluate(arg, Rc::clone(&env))?;
                        function_env.borrow_mut().define(param.lexeme.clone(), value);
                    }
                    
                    match execute_block(&body, function_env) {
                        Ok(_) => Ok(Value::Nil),
                        Err(RuntimeError::Return(value)) => Ok(value),
                        Err(e) => Err(e),
                    }
                }
                _ => Err(RuntimeError::new(
                    "Can only call functions.".to_string(),
                    paren.line,
                )),
            }
        }
    }
}
pub fn execute_stmt(stmt: &Stmt, print_expr_result: bool, env: Rc<RefCell<Environment>>) -> Result<(), RuntimeError> {
    match stmt {
        Stmt::Print(expr) => {
            let value = evaluate(expr, Rc::clone(&env))?;
            println!("{}", value);
            Ok(())
        }
        Stmt::Expression(expr) => {
            let value = evaluate(expr, Rc::clone(&env))?;
            if print_expr_result {
                println!("{}", value);
            }
            Ok(())
        }
        Stmt::Var(name, initializer) => {
            let value = match initializer {
                Some(expr) => evaluate(expr, Rc::clone(&env))?,
                None => Value::Nil,
            };
            env.borrow_mut().define(name.lexeme.clone(), value);
            Ok(())
        }
        Stmt::Block(statements) => {
            let block_env = Rc::new(RefCell::new(Environment::new_with_enclosing(Rc::clone(&env))));
            execute_block(statements, block_env)
        },
        Stmt::If(condition, then_branch, else_branch) => {
            let condition_value = evaluate(condition, Rc::clone(&env))?;
            if is_truthy(&condition_value) {
                execute_stmt(then_branch, print_expr_result, Rc::clone(&env))?;
            } else if let Some(else_stmt) = else_branch {
                execute_stmt(else_stmt, print_expr_result, Rc::clone(&env))?;
            }
            Ok(())
        },
        Stmt::While(condition, body) => {
            while is_truthy(&evaluate(condition, Rc::clone(&env))?) {
                execute_stmt(body, print_expr_result, Rc::clone(&env))?;
            }
            Ok(())
        },
        Stmt::Function(name, params, body) => {
            let function = Value::Function(
                name.lexeme.clone(), 
                params.clone(), 
                body.clone(), 
                Rc::clone(&env)
            );
            env.borrow_mut().define(name.lexeme.clone(), function);
            Ok(())
        },
        Stmt::Return(_, value) => {
            let return_value = match value {
                Some(expr) => evaluate(expr, env)?,
                None => Value::Nil,
            };
            Err(RuntimeError::Return(return_value))
        }
    }
}

fn execute_block(statements: &[Stmt], env: Rc<RefCell<Environment>>) -> Result<(), RuntimeError> {
    for statement in statements {
        execute_stmt(statement, false, Rc::clone(&env))?;
    }
    Ok(())
}

fn compare_equality(left: &Value, right: &Value) -> Result<bool, RuntimeError> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok((l - r).abs() < f64::EPSILON),
        (Value::String(l), Value::String(r)) => Ok(l == r),
        (Value::Boolean(l), Value::Boolean(r)) => Ok(l == r),
        (Value::Nil, Value::Nil) => Ok(true),
        _ => Ok(false),
    }
}

fn compare_values(left: &Value, right: &Value, compare: fn(f64, f64) -> bool) -> Result<Value, RuntimeError> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Boolean(compare(*l, *r))),
        _ => Err(RuntimeError::new("Operands must be numbers.".to_string(), 0)),
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Boolean(b) => *b,
        Value::Nil => false,
        _ => true,
    }
}


