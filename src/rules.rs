use crate::errors::*;

pub fn matches(req: &str, rules: &[String]) -> bool {
    for rule in rules {
        if rule == "*" {
            debug!("Request to {:?} matched wildcard", req);
            return true;
        } else if rule == req {
            debug!("Request to {:?} matched rule", req);
            return true;
        }
    }
    false
}
