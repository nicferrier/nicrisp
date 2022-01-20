use std::collections::HashMap;
use std::fmt;
use std::io;
use std::io::Write;
use std::num::ParseFloatError;
use std::rc::Rc;
use reqwest::blocking::get as httpget;

trait RispValueString {
  fn lisp_val(&self) -> String;
}

#[derive(Clone)]
enum RispExp {
  Bool(bool),
  Symbol(String),
  Number(f64),
  Str(String),
  List(Vec<RispExp>),
  Func(fn(&[RispExp]) -> Result<RispExp, RispErr>),
  Lambda(RispLambda),
}

#[derive(Clone)]
struct RispLambda {
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
enum RispErr {
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

  for c in expr.chars() {
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
      tokens.push(buf_str);
      buf_str = String::new();
      continue;
    }

    buf_str.push(c);
  }

  tokens.push(buf_str);

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
      let floats = parse_list_of_floats(args)?;
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
  data.insert(
    "httpget".to_string(),
    RispExp::Func(
      |args: &[RispExp]| -> Result<RispExp, RispErr> {
	if args.len() < 1 {
	  return Err(RispErr::Reason("pass a url".to_string()));
	}
	let url = args[0].lisp_val();
	let res = match httpget(url) {
	  Ok(response) => Box::new(response),
	  Err(e) => return Err(RispErr::Reason(e.to_string())),
	};

	let status = res.status().as_u16() as f64;
	let res_url = res.url();
	let headers = res.headers();
	let mut header_list: Vec<RispExp> = Vec::new();
	for (name, value) in headers.iter() {
	  let mut pair = Vec::new();
	  pair.push(RispExp::Str(name.to_string()));
	  pair.push(RispExp::Str(value.to_str().unwrap().to_string()));
	  header_list.push(RispExp::List(pair));
	}
	let response_list: Vec<RispExp> = vec![
	  RispExp::Number(status),
	  RispExp::Str(res_url.to_string()),
	  RispExp::List(header_list)
	];
	Ok(RispExp::List(response_list))
      }
    )
  );
  data.insert(
    "num".to_string(),
    RispExp::Func(
      |args: &[RispExp]| -> Result<RispExp, RispErr> {
	if args.len() < 1 {
	  return Err(RispErr::Reason("pass a max value".to_string()));
	}

	let max = match args[0] {
	  RispExp::Number(x) => x as i64,
	  _ => return Err(RispErr::Reason("arg is not a number".to_string())),
	};

	let start = if args.len() < 2 { 0 } else {
	  match args[1] {
	    RispExp::Number(x) => x as i64,
	    _ => return Err(RispErr::Reason("arg is not a number".to_string())),
	  }
	};

	let mut res: Vec<RispExp> = Vec::new();
	let r = start..max;
	for m in r {
	  res.push(RispExp::Number(m as f64));
	}
	Ok(RispExp::List(res))
      }
    )
  );
  data.insert(
    "+".to_string(), 
    RispExp::Func(
      |args: &[RispExp]| -> Result<RispExp, RispErr> {
        let sum = parse_list_of_floats(args)?.iter().fold(0.0, |sum, a| sum + a);
        Ok(RispExp::Number(sum))
      }
    )
  );
  data.insert(
    "-".to_string(), 
    RispExp::Func(
      |args: &[RispExp]| -> Result<RispExp, RispErr> {
        let floats = parse_list_of_floats(args)?;
        let first = *floats.first().ok_or(RispErr::Reason("expected at least one number".to_string()))?;
        let sum_of_rest = floats[1..].iter().fold(0.0, |sum, a| sum + a);
        
        Ok(RispExp::Number(first - sum_of_rest))
      }
    )
  );
  data.insert(
    "=".to_string(), 
    RispExp::Func(ensure_tonicity!(|a, b| a == b))
  );
  data.insert(
    ">".to_string(), 
    RispExp::Func(ensure_tonicity!(|a, b| a > b))
  );
  data.insert(
    ">=".to_string(), 
    RispExp::Func(ensure_tonicity!(|a, b| a >= b))
  );
  data.insert(
    "<".to_string(), 
    RispExp::Func(ensure_tonicity!(|a, b| a < b))
  );
  data.insert(
    "<=".to_string(), 
    RispExp::Func(ensure_tonicity!(|a, b| a <= b))
  );
  
  RispEnv {data, outer: None}
}

fn parse_list_of_floats(args: &[RispExp]) -> Result<Vec<f64>, RispErr> {
  args
    .iter()
    .map(|x| parse_single_float(x))
    .collect()
}

fn parse_single_float(exp: &RispExp) -> Result<f64, RispErr> {
  match exp {
    RispExp::Number(num) => Ok(*num),
    _ => Err(RispErr::Reason("expected a number".to_string())),
  }
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


fn eval_built_in_form(
  exp: &RispExp, arg_forms: &[RispExp], env: &mut RispEnv
) -> Option<Result<RispExp, RispErr>> {
  match exp {
    RispExp::Symbol(s) => 
      match s.as_ref() {
        "if" => Some(eval_if_args(arg_forms, env)),
        "def" => Some(eval_def_args(arg_forms, env)),
        "fn" => Some(eval_lambda_args(arg_forms)),
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
  }
}

/*
  Repl
*/

fn parse_eval(expr: String, env: &mut RispEnv) -> Result<RispExp, RispErr> {
  let (parsed_exp, _) = parse(&tokenize(expr))?;
  let evaled_exp = eval(&parsed_exp, env)?;

  Ok(evaled_exp)
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
	  Ok(res) => println!("=> {}", res),
	  Err(e) => match e {
            RispErr::Reason(msg) => println!("=> {}", msg),
	  },
	}
      },
      Err(_e) => break,
    }
  }
}

/* Local Variables: */
/* mode: rust */
/* rust-indent-offset: 2 */
/* End: */
