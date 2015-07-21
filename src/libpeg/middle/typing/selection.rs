// Copyright 2014 Pierre Talbot (IRCAM)

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The selection phase is used to identify the typing contexts of the future
//! parsing functions.
//!
//! It can be untyped, typed or both depending on the calling contexts.
//! The calling context of the start rule is `UnValued` if its type is unit and
//! is `Valued` otherwise.
//!
//! Semantics actions in an untyped context won't be called.

use middle::typing::ast::*;
use middle::typing::ast::EvaluationContext::*;

pub fn selection_phase(grammar: &mut Grammar)
{
  Selector::select(&mut grammar.rules, grammar.attributes.starting_rule.clone());
}

struct Selector
{
  visited: HashMap<Ident, Option<EvaluationContext>>,
  to_visit: Vec<(Ident, EvaluationContext)>
}

impl Selector
{
  pub fn select(rules: &mut HashMap<Ident, Rule>, start: Ident)
  {
    let mut selector = Selector::new(rules);
    selector.rules_dfs(rules, start);
  }

  fn new(rules: &HashMap<Ident, Rule>) -> Selector
  {
    let mut visited = HashMap::with_capacity(rules.len());
    for id in rules.keys() {
      visited.insert(id.clone(), None);
    }
    Selector {
      visited: visited,
      to_visit: vec![]
    }
  }

  fn rules_dfs(&mut self, rules: &mut HashMap<Ident, Rule>, start: Ident)
  {
    self.to_visit.push((start, Selector::context_of_start_rule(rules, start.clone())));
    while !self.to_visit.is_empty() {
      let (rule_id, context) = self.to_visit.pop().unwrap();
      self.visit_rule(rules, rule_id, context);
    }
  }

  fn context_of_start_rule(rules: &HashMap<Ident, Rule>, start: Ident)
    -> EvaluationContext
  {
    if rules[&start].def.ty.borrow().is_unit() {
      UnValued
    }
    else {
      Valued
    }
  }

  fn mark_if_not_visited(&mut self, rule_id: Ident, context: EvaluationContext) -> bool
  {
    let visited = self.visited[&rule_id];
    let new_visited = Some(visited.flat_merge(context));
    if new_visited != visited {
      *self.visited.get_mut(&rule_id).unwrap() = new_visited;
      true
    } else {
      false
    }
  }

  fn first_visit(&mut self, rule_id: Ident) -> bool
  {
    self.visited[&rule_id].is_none()
  }

  fn visit_rule(&mut self, rules: &mut HashMap<Ident, Rule>,
    rule_id: Ident, context: EvaluationContext)
  {
    let first_visit = self.first_visit(rule_id);
    if self.mark_if_not_visited(rule_id, context) {
      let rule = rules.get_mut(&rule_id).unwrap();
      let to_visit = ExpressionVisitor::visit(&mut rule.def, context, first_visit);
      self.to_visit.push_all(&to_visit[..]);
    }
  }
}

struct ExpressionVisitor
{
  to_visit: Vec<(Ident, EvaluationContext)>,
  first_visit: bool
}

impl ExpressionVisitor
{
  fn visit(expr: &mut Expression, context: EvaluationContext, first_visit: bool)
    -> Vec<(Ident, EvaluationContext)>
  {
    let mut visitor = ExpressionVisitor {
      to_visit: vec![],
      first_visit: first_visit
    };
    visitor.visit_expr(expr, context);
    visitor.to_visit
  }

  fn visit_expr(&mut self, expr: &mut Expression, mut context: EvaluationContext)
  {
    // The context of a () type doesn't change, so it's safe to return.
    if expr.ty.borrow().is_unit() {
      if self.first_visit {
        context = UnValued;
      }
      else {
        return ();
      }
    }
    expr.ty_context = expr.ty_context.merge(context);
    self.visit_expr_node(&mut expr.node, context);
  }

  fn visit_non_terminal_symbol(&mut self, ident: Ident, context: EvaluationContext)
  {
    self.to_visit.push((ident, context));
  }

  fn visit_expr_node(&mut self, expr: &mut ExpressionNode, context: EvaluationContext)
  {
    match expr {
      &mut NonTerminalSymbol(id) => {
        self.visit_non_terminal_symbol(id, context)
      }
        &mut Sequence(ref mut exprs)
      | &mut Choice(ref mut exprs) => {
        self.visit_exprs(&mut *exprs, context)
      }
        &mut ZeroOrMore(ref mut expr)
      | &mut OneOrMore(ref mut expr)
      | &mut Optional(ref mut expr)
      | &mut SemanticAction(ref mut expr, _) => {
        self.visit_expr(&mut *expr, context)
      }
        &mut NotPredicate(ref mut expr)
      | &mut AndPredicate(ref mut expr) => {
        self.visit_expr(&mut *expr, UnValued)
      }
        &mut StrLiteral(_)
      | &mut AnySingleChar
      | &mut CharacterClass(_) => ()
    }
  }

  fn visit_exprs(&mut self, exprs: &mut Vec<Box<Expression>>, context: EvaluationContext)
  {
    assert!(exprs.len() > 0);
    for expr in exprs.iter_mut() {
      self.visit_expr(&mut *expr, context);
    }
  }
}
