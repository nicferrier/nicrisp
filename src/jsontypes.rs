use serde_json;
use super::RispExp;
use super::RispErr;

pub fn display(data: &serde_json::Value) -> String {
    format!("{}", serde_json::to_string_pretty(data).unwrap())
}

pub fn get_func() -> RispExp {
    RispExp::Func(
	|args: &[RispExp]| -> Result<RispExp, RispErr> {
	    if args.len() < 2 {
		return Err(RispErr::Reason("pass a json object and an index".to_string()));
	    }

	    let index = match &args[1] {
		RispExp::Str(s) => s.to_string(),
		RispExp::Number(n) => n.to_string(),
		_ => return Err(RispErr::Reason("index must be string or number".to_string()))
	    };
	    match &args[0] {
		RispExp::Json(data) => Ok(RispExp::Json(data[index].clone())),
		_ => Err(RispErr::Reason("not a json object".to_string()))
	    }
	}
    )
}

// End
