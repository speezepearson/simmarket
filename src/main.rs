/*
Experiments to run:

Plot the supply and demand curves at:
- start state
- no-trades-left state

Check whether there's a relationship between the final spread and the supply/demand curves' intersection point.

Try imposing a price floor (or cap) and check whether what we get is readily relatable to the supply/demand curves we plotted.
Try the above for both binding and non-binding floor (cap).
*/

use rand::Rng;
use rand::rngs::StdRng;
use rand::distributions::{Distribution, Uniform};
use std::collections::HashMap;
use std::cmp::{min, max};
use rand::SeedableRng;

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let seed: u64 = args[1].parse::<u64>().unwrap();

  let mut rng: StdRng = StdRng::seed_from_u64(seed);

  let mut agents = Vec::new();

  println!("setting up agent pool");
  for i in 0..1000 {
    agents.push(Agent::new_random(&mut rng));
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

  println!("done with main");
}

#[derive(PartialEq, Debug, Copy, Clone)]
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

  fn new_random(rng: &mut StdRng) -> Agent {
    let prod_dist = Uniform::new(0.0,1000.0);
    let coeff_dist = Uniform::new(0.0,1.0);
    
    return Agent {
      production_a: prod_dist.sample(rng),
      production_b: prod_dist.sample(rng),

      consumption_a_coeff: coeff_dist.sample(rng),
      consumption_b_coeff: coeff_dist.sample(rng),
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
        amount_b: 0.12195121951219513,
      }
    );

    execute_one_trade(&mut assets);

    assert_eq!(
      find_next_trade(&assets).unwrap(),
      Trade{
        buyer: 1,
        seller: 0,
        amount_a: 0.5,
        amount_b: 0.12195121951219513,
      }
    );
  }

}

#[derive(PartialEq, Debug, Copy, Clone)]
struct Balance {
  a: f64,
  b: f64,
}

type AgentId = usize;

#[derive(Debug, PartialEq)]
struct Trade {
  buyer: AgentId,
  seller: AgentId,

  amount_a: f64, // transferred from seller to buyer
  amount_b: f64, // transferred from buyer to seller
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
      let (buyer, buyer_balance) = &assets[bid.agent_id];
      let (seller, seller_balance) = &assets[ask.agent_id];
      println!("  (balances: bidder {:?}, seller {:?})", buyer_balance, seller_balance);
      let clearing_price = (bid.price_per_a_in_b + ask.price_per_a_in_b) / 2.0;
      let amount_a_buyer_can_afford = buyer_balance.b / clearing_price;
      let (amount_a, amount_b) = if amount_a_buyer_can_afford < seller_balance.a {
        // amount_a_buyer_can_afford is known to be < seller_balance.a due to the if
        // statement above
        (amount_a_buyer_can_afford, buyer_balance.b)
      } else {
        (seller_balance.a, clearing_price * seller_balance.a)
      };
      return Some(Trade {
        buyer: bid.agent_id,
        seller: ask.agent_id,
        amount_a: amount_a,
        amount_b: amount_b,
      });
    }
    _ => { return None; }
  }    
}

fn execute_one_trade(assets: &mut Vec<(Agent, Balance)>) -> bool /* done? */ {
  println!("in execute_one_trade");
  match find_next_trade(assets) {
    None => { 
      println!("no more trades are possible");
      return true;
    }
    Some(trade) => {
      println!("executing {:?}", trade);
      let (initial_buyer_utility, initial_seller_utility) = {
        let (buyer, mut buyer_balance) = assets[trade.buyer];
        let (seller, mut seller_balance) = assets[trade.seller];
        (
          buyer.utility(buyer_balance.a, buyer_balance.b),
          seller.utility(seller_balance.a, seller_balance.b)
        )
      };
      assets[trade.buyer] .1.a += trade.amount_a; if assets[trade.buyer] .1.a < 0.0 {panic!("oh no")}
      assets[trade.seller].1.a -= trade.amount_a; if assets[trade.seller].1.a < 0.0 {panic!("oh no")}
      assets[trade.buyer] .1.b -= trade.amount_b; if assets[trade.buyer] .1.b < 0.0 {panic!("oh no")}
      assets[trade.seller].1.b += trade.amount_b; if assets[trade.seller].1.b < 0.0 {panic!("oh no")}
      let (final_buyer_utility, final_seller_utility) = {
        let (buyer, mut buyer_balance) = assets[trade.buyer];
        let (seller, mut seller_balance) = assets[trade.seller];
        (
          buyer.utility(buyer_balance.a, buyer_balance.b),
          seller.utility(seller_balance.a, seller_balance.b)
        )
      };
      // println!("buyer {:?}", buyer);
      // println!("  util {} -> {}", initial_buyer_utility, final_buyer_utility);
      // println!("seller {:?}", seller);
      // println!("  util {} -> {}", initial_seller_utility, final_seller_utility);
      assert!(final_buyer_utility > initial_buyer_utility, "buyer's remorse");
      assert!(final_seller_utility > initial_seller_utility, "seller's remorse");
      return false;
    }
  }
}

fn execute_all_trades(assets: &mut Vec<(Agent, Balance)>) {
  while !execute_one_trade(assets) {}
  sanity_check_endpoint(assets);
}

fn sanity_check_endpoint(assets: &Vec<(Agent, Balance)>) {
  let mut local = assets.clone();
  local.sort_by(|(agent_1,_), (agent_2, _)| {
    agent_1.indifference_price_of_a_in_b().partial_cmp(
      &agent_2.indifference_price_of_a_in_b()
    ).unwrap()
  });

  let remainder = local.iter()
    .skip_while(|(_, balance)| {    balance.a == 0.0  })
    .skip_while(|(_, balance)| {    balance.a > 0.0 && balance.b > 0.0  })
    .skip_while(|(_, balance)| {    balance.b == 0.0  })
    .collect::<Vec<_>>();
  // println!("Agents:");
  // for (agent, balance) in local.iter() {
  //   println!("  ({}, {}, {}), {:?}", agent.indifference_price_of_a_in_b(), balance.a, balance.b, agent);
  // }
  // println!("Remainder:");
  // for (agent, balance) in remainder.iter() {
  //   println!("  ({}, {}, {}), {:?}", agent.indifference_price_of_a_in_b(), balance.a, balance.b, agent);
  // }
  assert!(remainder.is_empty(), "{:?} ({} elems)", remainder, remainder.len());
}
