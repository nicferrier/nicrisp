use super::RispExp;
use super::RispErr;

pub fn number_sequence() -> RispExp {
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
}

// End
