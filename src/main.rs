use std::collections::HashMap;
use std::fmt;
use std::io;
use std::io::Write;
use std::num::ParseFloatError;
use std::rc::Rc;
use serde_json;

trait RispValueString {
  fn lisp_val(&self) -> String;
}

#[derive(Clone)]
pub enum RispExp {
  Bool(bool),
  Symbol(String),
  Number(f64),
  Str(String),
  List(Vec<RispExp>),
  Func(fn(&[RispExp]) -> Result<RispExp, RispErr>),
  Lambda(RispLambda),
  Json(serde_json::Value)
}

mod lists;
mod math;
mod http;
mod jsontypes;

#[derive(Clone)]
pub struct RispLambda {
  params_exp: Rc<RispExp>,
  body_exp: Rc<RispExp>,
}

// Conventional rust to_string used for printable form
impl fmt::Display for RispExp {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let str = match self {
      RispExp::Bool(a) => a.to_string(),
      RispExp::Symbol(s) => s.clone(),
      RispExp::Number(n) => n.to_string(),
      RispExp::Str(s) => format!("\"{}\"", s),
      RispExp::List(list) => {
        let xs: Vec<String> = list
          .iter()
          .map(|x| match x {
	    RispExp::Str(s) => format!("\"{}\"", s.to_string()),
	    _ => x.to_string(),
	  })
          .collect();
        format!("({})", xs.join(","))
      },
      RispExp::Func(_) => "Function {}".to_string(),
      RispExp::Lambda(_) => "Lambda {}".to_string(),
      RispExp::Json(data) => jsontypes::display(&data),
    };
    
    write!(f, "{}", str)
  }
}

// For the internal value, not the printable form
impl RispValueString for RispExp {
  fn lisp_val(&self) -> String {
    match self {
      RispExp::Str(s) => s.clone(),
      _ => self.to_string()
    }
  }
}

#[derive(Debug)]
pub enum RispErr {
  Reason(String),
}

#[derive(Clone)]
struct RispEnv<'a> {
  data: HashMap<String, RispExp>,
  outer: Option<&'a RispEnv<'a>>,
}


/*
  Parse
*/

fn tokenize(expr: String) -> Vec<String> {
  let mut tokens = Vec::new();
  let mut buf_str = String::new();
  let mut in_quote = false;
  let mut in_comment = false;

  for c in expr.chars() {
    if in_comment && c != '\n' {
      continue;
    }

    if in_comment && c == '\n' {
      in_comment = false;
      continue;
    }

    if (c == ';' || c == '#') && !in_quote {
      in_comment = true;
      continue;
    }
    
    if c == '"' && in_quote {
      buf_str.push('"');
      tokens.push(buf_str);
      in_quote = false;
      buf_str = String::new();
      continue;
    }

    if c == '"' && !in_quote {
      in_quote = true;
      if buf_str.len() > 0 {
	tokens.push(buf_str);
      }
      buf_str = String::from("\"");
      continue;
    }

    if in_quote {
      buf_str.push(c);
      continue;
    }

    if c == '(' || c == ')' {
      if buf_str.len() > 0 {
	tokens.push(buf_str);
	buf_str = String::new();
      }
      tokens.push(c.to_string());
      continue;
    }

    if c == ' ' || c == '\n' {
      if buf_str.len() > 0 {
	tokens.push(buf_str);
	buf_str = String::new();
      }
      continue;
    }

    buf_str.push(c);
  }

  if buf_str.len() > 0 {
    tokens.push(buf_str);
  }

  if false {
    for token in tokens.iter() {
      println!("token {}", token);
    }
  }

  tokens
}

fn parse<'a>(tokens: &'a [String]) -> Result<(RispExp, &'a [String]), RispErr> {
  let (token, rest) = tokens.split_first()
    .ok_or(
      RispErr::Reason("could not get token".to_string())
    )?;
  match &token[..] {
    "(" => read_seq(rest),
    ")" => Err(RispErr::Reason("unexpected `)`".to_string())),
    _ => Ok((parse_atom(token), rest)),
  }
}

fn read_seq<'a>(tokens: &'a [String]) -> Result<(RispExp, &'a [String]), RispErr> {
  let mut res: Vec<RispExp> = vec![];
  let mut xs = tokens;
  loop {
    let (next_token, rest) = xs
      .split_first()
      .ok_or(RispErr::Reason("could not find closing `)`".to_string()))
      ?;
    if next_token == ")" {
      return Ok((RispExp::List(res), rest)) // skip `)`, head to the token after
    }
    let (exp, new_xs) = parse(&xs)?;
    res.push(exp);
    xs = new_xs;
  }
}

fn parse_atom(token: &str) -> RispExp {
  match token.as_ref() {
    "true" => RispExp::Bool(true),
    "false" => RispExp::Bool(false),
    _ => {
      if token.len() > 0 && token.chars().nth(0).unwrap() == '"' {
	let s = token.to_string();
	let val = &s[1..s.len() - 1];
	return RispExp::Str(val.to_string());
      }
      let potential_float: Result<f64, ParseFloatError> = token.parse();
      match potential_float {
        Ok(v) => RispExp::Number(v),
        Err(_) => RispExp::Symbol(token.to_string().clone())
      }
    }
  }
}

/*
  Env
*/

macro_rules! ensure_tonicity {
  ($check_fn:expr) => {{
    |args: &[RispExp]| -> Result<RispExp, RispErr> {
      let floats = math::parse_list_of_floats(args)?;
      let first = floats.first().ok_or(RispErr::Reason("expected at least one number".to_string()))?;
      let rest = &floats[1..];
      fn f (prev: &f64, xs: &[f64]) -> bool {
        match xs.first() {
          Some(x) => $check_fn(prev, x) && f(x, &xs[1..]),
          None => true,
        }
      }
      Ok(RispExp::Bool(f(first, rest)))
    }
  }};
}

fn default_env<'a>() -> RispEnv<'a> {
  let mut data: HashMap<String, RispExp> = HashMap::new();
  data.insert("httpget".to_string(), http::httpget_func());
  data.insert("num".to_string(), lists::number_sequence());
  data.insert("*".to_string(), math::mult_func());
  data.insert("+".to_string(), math::plus_func());
  data.insert("-".to_string(), math::minus_func());
  data.insert("=".to_string(), RispExp::Func(ensure_tonicity!(|a, b| a == b)));
  data.insert(">".to_string(), RispExp::Func(ensure_tonicity!(|a, b| a > b)));
  data.insert(">=".to_string(), RispExp::Func(ensure_tonicity!(|a, b| a >= b)));
  data.insert("<".to_string(), RispExp::Func(ensure_tonicity!(|a, b| a < b)));
  data.insert("<=".to_string(), RispExp::Func(ensure_tonicity!(|a, b| a <= b)));
  RispEnv {data, outer: None}
}

/*
  Eval
*/

fn eval_if_args(arg_forms: &[RispExp], env: &mut RispEnv) -> Result<RispExp, RispErr> {
  let test_form = arg_forms.first().ok_or(
    RispErr::Reason(
      "expected test form".to_string(),
    )
  )?;
  let test_eval = eval(test_form, env)?;
  match test_eval {
    RispExp::Bool(b) => {
      let form_idx = if b { 1 } else { 2 };
      let res_form = arg_forms.get(form_idx)
        .ok_or(RispErr::Reason(
          format!("expected form idx={}", form_idx)
        ))?;
      let res_eval = eval(res_form, env);
      
      res_eval
    },
    _ => Err(
      RispErr::Reason(format!("unexpected test form='{}'", test_form.to_string()))
    )
  }
}

fn eval_def_args(arg_forms: &[RispExp], env: &mut RispEnv) -> Result<RispExp, RispErr> {
  let first_form = arg_forms.first().ok_or(
    RispErr::Reason(
      "expected first form".to_string(),
    )
  )?;
  let first_str = match first_form {
    RispExp::Symbol(s) => Ok(s.clone()),
    _ => Err(RispErr::Reason(
      "expected first form to be a symbol".to_string(),
    ))
  }?;
  let second_form = arg_forms.get(1).ok_or(
    RispErr::Reason(
      "expected second form".to_string(),
    )
  )?;
  if arg_forms.len() > 2 {
    return Err(
      RispErr::Reason(
        "def can only have two forms ".to_string(),
      )
    )
  } 
  let second_eval = eval(second_form, env)?;
  env.data.insert(first_str, second_eval);
  
  Ok(first_form.clone())
}


fn eval_lambda_args(arg_forms: &[RispExp]) -> Result<RispExp, RispErr> {
  let params_exp = arg_forms.first().ok_or(
    RispErr::Reason(
      "expected args form".to_string(),
    )
  )?;
  let body_exp = arg_forms.get(1).ok_or(
    RispErr::Reason(
      "expected second form".to_string(),
    )
  )?;
  if arg_forms.len() > 2 {
    return Err(
      RispErr::Reason(
        "fn definition can only have two forms ".to_string(),
      )
    )
  }
  
  Ok(
    RispExp::Lambda(
      RispLambda {
        body_exp: Rc::new(body_exp.clone()),
        params_exp: Rc::new(params_exp.clone()),
      }
    )
  )
}

fn eval_repeat_args(arg_forms: &[RispExp], env: &mut RispEnv) -> Result<RispExp, RispErr> {
  let (func_form, rest) = arg_forms.split_first().ok_or(
    RispErr::Reason(
      "expected function form".to_string(),
    )
  )?;
  let lambda = eval(func_form, env)?;
  let lambda = match lambda {
    RispExp::Lambda(f) => f,
    _ => return Err(RispErr::Reason("not a function".to_string()))
  };
  let list_form = rest.first().ok_or(RispErr::Reason("expected list".to_string()))?;
  let list_val =  eval(list_form, env)?;
  match list_val {
    RispExp::List(l) => {
      let mut result_vec = Vec::new();
      for risp_val in l {
	let args = &[risp_val];
	let new_env = &mut env_for_lambda(lambda.params_exp.clone(), args, env)?;
        let result_val = eval(&lambda.body_exp, new_env)?;
	result_vec.push(result_val);
      }
      Ok(RispExp::List(result_vec))
    },
    _ => Err(RispErr::Reason("not a list".to_string()))
  }
}

fn eval_built_in_form(
  exp: &RispExp, arg_forms: &[RispExp], env: &mut RispEnv
) -> Option<Result<RispExp, RispErr>> {
  match exp {
    RispExp::Symbol(s) => 
      match s.as_ref() {
        "if" => Some(eval_if_args(arg_forms, env)),
        "def" => Some(eval_def_args(arg_forms, env)),
        "fn" => Some(eval_lambda_args(arg_forms)),
	"repeat" => Some(eval_repeat_args(arg_forms, env)),
        _ => None,
      }
    ,
    _ => None,
  }
}

fn env_get(k: &str, env: &RispEnv) -> Option<RispExp> {
  if false {
    for (key, value) in &env.data {
      println!("env key {}: {}", key, value);
    }
  }

  // Self quoted symbols just resolve to themselves
  if k.starts_with(":") {
    return Some(RispExp::Symbol(k.to_string()));
  }

  match env.data.get(k) {
    Some(exp) => Some(exp.clone()),
    None => {
      match &env.outer {
        Some(outer_env) => env_get(k, &outer_env),
        None => None
      }
    }
  }
}

fn parse_list_of_symbol_strings(form: Rc<RispExp>) -> Result<Vec<String>, RispErr> {
  let list = match form.as_ref() {
    RispExp::List(s) => Ok(s.clone()),
    _ => Err(RispErr::Reason(
      "expected args form to be a list".to_string(),
    ))
  }?;
  list
    .iter()
    .map(
      |x| {
        match x {
          RispExp::Symbol(s) => Ok(s.clone()),
          _ => Err(RispErr::Reason(
            "expected symbols in the argument list".to_string(),
          ))
        }   
      }
    ).collect()
}

fn env_for_lambda<'a>(
  params: Rc<RispExp>, 
  arg_forms: &[RispExp],
  outer_env: &'a mut RispEnv,
) -> Result<RispEnv<'a>, RispErr> {
  let ks = parse_list_of_symbol_strings(params)?;
  if ks.len() != arg_forms.len() {
    return Err(
      RispErr::Reason(
        format!("expected {} arguments, got {}", ks.len(), arg_forms.len())
      )
    );
  }
  let vs = eval_forms(arg_forms, outer_env)?;
  let mut data: HashMap<String, RispExp> = HashMap::new();
  for (k, v) in ks.iter().zip(vs.iter()) {
    data.insert(k.clone(), v.clone());
  }
  Ok(
    RispEnv {
      data,
      outer: Some(outer_env),
    }
  )
}

fn eval_forms(arg_forms: &[RispExp], env: &mut RispEnv) -> Result<Vec<RispExp>, RispErr> {
  arg_forms
    .iter()
    .map(|x| eval(x, env))
    .collect()
}

fn eval(exp: &RispExp, env: &mut RispEnv) -> Result<RispExp, RispErr> {
  match exp {
    RispExp::Symbol(k) =>
      env_get(k, env)
      .ok_or(
        RispErr::Reason(
          format!("unexpected symbol k='{}'", k)
        )
      ),
    RispExp::Str(_a) => Ok(exp.clone()),
    RispExp::Bool(_a) => Ok(exp.clone()),
    RispExp::Number(_a) => Ok(exp.clone()),

    RispExp::List(list) => {
      let first_form = list
        .first()
        .ok_or(RispErr::Reason("expected a non-empty list".to_string()))?;
      let arg_forms = &list[1..];
      match eval_built_in_form(first_form, arg_forms, env) {
        Some(res) => res,
        None => {
          let first_eval = eval(first_form, env)?;
          match first_eval {
            RispExp::Func(f) => {
              f(&eval_forms(arg_forms, env)?)
            },
            RispExp::Lambda(lambda) => {
              let new_env = &mut env_for_lambda(lambda.params_exp, arg_forms, env)?;
              eval(&lambda.body_exp, new_env)
            },
            _ => Err(
              RispErr::Reason("first form must be a function".to_string())
            ),
          }
        }
      }
    },
    RispExp::Func(_) => Err(RispErr::Reason("unexpected form".to_string())),
    RispExp::Lambda(_) => Err(RispErr::Reason("unexpected form".to_string())),
    RispExp::Json(_) => Ok(exp.clone()),
  }
}

/*
  Repl
*/

fn parse_eval(expr: String, env: &mut RispEnv) -> Option<Result<RispExp, RispErr>> {
  let tokens = &tokenize(expr);
  if tokens.len() < 1 {
    return None;
  }

  let (parsed_exp, _) = parse(tokens).unwrap();
  match eval(&parsed_exp, env) {
    Ok(evaled_exp) => Some(Ok(evaled_exp)),
    Err(e) => Some(Err(e))
  }
}

#[derive(Debug)]
enum RispIOErr {
  Reason(String),
}
  
fn slurp_expr() -> Result<String, RispIOErr> {
  let mut expr = String::new();
  
  let red = io::stdin().read_line(&mut expr)
    .expect("Failed to read line");

  if red == 0 {
    println!(); // newline to clear up the terminal
    return Err(RispIOErr::Reason("EOF".to_string()));
  }

  Ok(expr)
}

fn main() {
  let env = &mut default_env();
  loop {
    print!("risp> ");
    io::stdout().flush().unwrap();
    match slurp_expr() {
      Ok(expr) => {
	match parse_eval(expr, env) {
	  Some(res) => match res {
	    Ok(res) => println!("=> {}", res),
	    Err(e) => match e {
              RispErr::Reason(msg) => println!("=> {}", msg),
	    },
	  },
	  None => println!(""),
	};
      },
      Err(_e) => break,
    }
  }
}

/* Local Variables: */
/* mode: rust */
/* rust-indent-offset: 2 */
/* End: */
