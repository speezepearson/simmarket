/*
Experiments to run:

Plot the supply and demand curves at:
- start state
- no-trades-left state

Check whether there's a relationship between the final spread and the supply/demand curves' intersection point.

Try imposing a price floor (or cap) and check whether what we get is readily relatable to the supply/demand curves we plotted.
Try the above for both binding and non-binding floor (cap).
*/

// extern crate rand;
use rand::Rng;
use rand::distributions::{Distribution, Uniform};
use std::collections::HashMap;
use std::cmp::{min, max};

fn main() {
  let mut agents = Vec::new();

  println!("setting up agent pool");
  for i in 0..1000 {
    agents.push(Agent::new_random());
  }

  let mut assets = Vec::new();
  for agent in agents {
    let a = agent.production_a;
    let b = agent.production_b;
    assets.push(
      (
        agent,
        Balance{
          a: a,
          b: b,
        }
      )
    );
  }

  execute_all_trades(&mut assets);
}

#[derive(PartialEq, Debug)]
struct Agent {
    // Production ability per time unit of each commodity
    production_a: f64,
    production_b: f64,

    consumption_a_coeff: f64,
    consumption_b_coeff: f64,
}

impl Agent {
  fn utility(&self, consumption_a: f64, consumption_b: f64) -> f64 {
    return self.consumption_a_coeff*consumption_a + self.consumption_b_coeff*consumption_b
  }

  fn indifference_price_of_a_in_b(&self) -> f64 {
    return self.consumption_a_coeff / self.consumption_b_coeff;
  }

  fn new_random() -> Agent {
    let prod_dist = Uniform::new(0.0,1000.0);
    let coeff_dist = Uniform::new(0.0,1.0);
    let mut rng = rand::thread_rng();
    
    return Agent {
      production_a: prod_dist.sample(&mut rng),
      production_b: prod_dist.sample(&mut rng),

      consumption_a_coeff: coeff_dist.sample(&mut rng),
      consumption_b_coeff: coeff_dist.sample(&mut rng),
    }
  }
}

mod tests {
  use crate::*;

  #[test]
  fn test_indifference_price() {
    let agent = Agent {
      production_a: 10.0,
      production_b: 10.0,
      consumption_a_coeff: 1.0,
      consumption_b_coeff: 5.0,
    };
    assert_eq!(agent.indifference_price_of_a_in_b(), 0.20);

    let price_a_in_b = agent.indifference_price_of_a_in_b();
    let amount_a_bought = 1.0;

    let consumption_a = agent.production_a + amount_a_bought;
    let consumption_b = agent.production_b - amount_a_bought*price_a_in_b;


    assert_eq!(
      agent.utility(agent.production_a, agent.production_b),
      agent.utility(consumption_a, consumption_b),
    );
  }

  #[test]
  fn test_find_next_trade() {
    let mut assets = vec![
      (
        Agent {
          production_a: 0.0,
          production_b: 0.0,
          consumption_a_coeff: 1.0,
          consumption_b_coeff: 5.0,
        },
        Balance {
          a: 1.0,
          b: 2.0,
        },
      ),
      (
        Agent {
          production_a: 0.0,
          production_b: 0.0,
          consumption_a_coeff: 8.0,
          consumption_b_coeff: 1.0,
        },
        Balance {
          a: 3.0,
          b: 4.0,
        },
      ),
    ];

    assert_eq!(
      find_next_trade(&assets).unwrap(),
      Trade{
        buyer: 1,
        seller: 0,
        amount_a: 0.5,
        price_per_a_in_b: 4.1,
      }
    );

    execute_one_trade(&mut assets);

    assert_eq!(
      find_next_trade(&assets).unwrap(),
      Trade{
        buyer: 1,
        seller: 0,
        amount_a: 0.5,
        price_per_a_in_b: 4.1,
      }
    );
  }

}

struct Balance {
  a: f64,
  b: f64,
}

type AgentId = usize;

#[derive(Debug, PartialEq)]
struct Trade {
  buyer: AgentId,
  seller: AgentId,

  amount_a: f64,
  price_per_a_in_b: f64,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum OrderType {
  Bid,
  Ask,
}


#[derive(PartialEq, Debug, Copy, Clone)]
struct Order {
  agent_id: AgentId,
  
  typ: OrderType,

  amount_a: f64,
  price_per_a_in_b: f64,
}

fn generate_orders(agent_id: AgentId, agent: &Agent, balance: &Balance) -> (Option<Order>, Option<Order>) {
  let bid = {
    if balance.b > 0.0 {
      Some(Order {
        agent_id: agent_id,
        typ: OrderType::Bid,
        amount_a: balance.b / agent.indifference_price_of_a_in_b(),
        price_per_a_in_b: agent.indifference_price_of_a_in_b(),
      })
    } else {
      None
    }
  };

  let ask = {
    if balance.a > 0.0 {
      Some(Order {
        agent_id: agent_id,
        typ: OrderType::Ask,
        amount_a: balance.a,
        price_per_a_in_b: agent.indifference_price_of_a_in_b(),
      })
    } else {
      None
    }
  };

  return (bid, ask)
}

fn find_next_trade(assets : &Vec<(Agent, Balance)>) -> Option<Trade> {
  let orders: Vec<(Option<Order>, Option<Order>)> =
    assets.iter().enumerate()
    .map(|(id, (agent, balance))| generate_orders(id, agent, balance))
    .collect();

  let highest_bid = orders.iter()
    .map(|(bid, _)| bid)
    .filter(|o| o.is_some()).map(|o| o.unwrap())
    .max_by(|o1, o2| o1.price_per_a_in_b.partial_cmp(&o2.price_per_a_in_b).unwrap());
  let lowest_acceptable_ask = orders.iter()
    .map(|(_, ask)| ask)
    .filter(|o| o.is_some()).map(|o| o.unwrap())
    .filter(|o| !highest_bid.is_some() || o.price_per_a_in_b < highest_bid.unwrap().price_per_a_in_b)
    .min_by(|o1, o2| o1.price_per_a_in_b.partial_cmp(&o2.price_per_a_in_b).unwrap());

  match (highest_bid, lowest_acceptable_ask) {
    (Some(bid), Some(ask)) => { 
      println!("matching bid {:?} against ask {:?}", bid, ask);
      let clearing_price = (bid.price_per_a_in_b + ask.price_per_a_in_b) / 2.0;
      return Some(Trade {
        buyer: bid.agent_id,
        seller: ask.agent_id,
        amount_a: if bid.amount_a < ask.amount_a { bid.amount_a } else { ask.amount_a },
        price_per_a_in_b: clearing_price,
      });
    }
    _ => { return None; }
  }    
}

fn execute_one_trade(assets: &mut Vec<(Agent, Balance)>) -> bool /* done? */ {
  match find_next_trade(assets) {
    None => { return true; }
    Some(trade) => {
      println!("executing {:?}", trade);
      assets[trade.buyer] .1.a += trade.amount_a;
      assets[trade.seller].1.a -= trade.amount_a;
      assets[trade.buyer] .1.b -= trade.amount_a * trade.price_per_a_in_b;
      assets[trade.seller].1.b += trade.amount_a * trade.price_per_a_in_b;
      return false;
    }
  }
}

fn execute_all_trades(assets: &mut Vec<(Agent, Balance)>) {
  while execute_one_trade(assets) {}
}


