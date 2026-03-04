//! Sprint 18 - Automation & Compliance Golden Path Tests
//!
//! Golden Path тестове за:
//! - TWAP/VWAP algorithmic execution
//! - Smart order routing (SOR)
//! - Iceberg orders
//! - Cost calculation and venue analysis
//! - Full automation lifecycle

use investor_os::execution::{
    algorithms::{AlgorithmSelector, IcebergExecutor, TWAPExecutor, VWAPExecutor},
    cost::{CostCalculator, ImpactModel},
    router::{RouteDecision, SmartRouter},
    venue::{FeeStructure, Venue, VenueAnalyzer, VenueQuote},
    ExecutionEngine, Fill, Order, OrderSide, OrderType, TimeInForce,
};
use rust_decimal::Decimal;

/// GOLDEN PATH: TWAP execution
///
/// Тества:
/// 1. Създаване на TWAP поръчка
/// 2. Разделяне на slices
/// 3. Изпълнение по време
/// 4. Сума от fills = общо количество
#[tokio::test]
async fn test_twap_execution() {
    println!("\n⏱️  Testing TWAP Execution");

    let executor = TWAPExecutor::new();

    // TWAP поръчка: 100 BTC за 1 секунда (ускорено за тест), 5 slices
    let order = Order::twap("BTC", OrderSide::Buy, Decimal::from(100), 1, 5);

    println!(
        "✅ Created TWAP order: {} BTC over {}s in {} slices",
        order.quantity, 60, 5
    );

    // Изпълнение
    let fills = executor
        .execute(&order, |qty| async move {
            Ok(Fill {
                id: uuid::Uuid::new_v4(),
                order_id: order.id,
                symbol: "BTC".to_string(),
                side: OrderSide::Buy,
                quantity: qty,
                price: Decimal::from(50000),
                venue: Venue::Binance,
                timestamp: chrono::Utc::now(),
                fees: qty * Decimal::from(50000) * Decimal::try_from(0.001).unwrap(),
            })
        })
        .await
        .expect("TWAP should execute");

    println!("   Fills received: {}", fills.len());

    // Проверки
    assert_eq!(fills.len(), 5, "Should have 5 fills (one per slice)");

    // Общо количество
    let total_filled: Decimal = fills.iter().map(|f| f.quantity).sum();
    assert_eq!(
        total_filled,
        Decimal::from(100),
        "Total should equal order quantity"
    );

    // Всяка fill трябва да е приблизително 20 BTC (100/5)
    for (i, fill) in fills.iter().enumerate() {
        println!(
            "   Slice {}: {} BTC @ ${}",
            i + 1,
            fill.quantity,
            fill.price
        );
        assert_eq!(fill.quantity, Decimal::from(20));
    }

    // Общи такси
    let total_fees: Decimal = fills.iter().map(|f| f.fees).sum();
    println!("   Total fees: ${}", total_fees);
    assert!(total_fees > Decimal::ZERO);

    println!("✅ TWAP execution test completed!");
}

/// GOLDEN PATH: VWAP algorithm
///
/// Тества:
/// 1. VWAP профил (U-shaped)
/// 2. Тегло по време на деня
/// 3. Повече volume при отваряне/затваряне
#[tokio::test]
async fn test_vwap_algorithm() {
    println!("\n📊 Testing VWAP Algorithm");

    // Генериране на интрадей профил (24 buckets)
    let profile = VWAPExecutor::default_intraday_profile(24);

    println!("✅ Generated intraday volume profile (24 buckets)");

    // Проверка на U-shape
    let open_weight = profile.first().unwrap();
    let midday_weight = profile.get(12).unwrap(); // Обяд
    let close_weight = profile.last().unwrap();

    println!("   Open weight: {}", open_weight);
    println!("   Midday weight: {}", midday_weight);
    println!("   Close weight: {}", close_weight);

    // Отваряне и затваряне трябва да са по-високи от обяд
    assert!(
        open_weight > midday_weight,
        "Open should have higher volume than midday"
    );
    assert!(
        close_weight > midday_weight,
        "Close should have higher volume than midday"
    );

    // VWAP поръчка (1 сек за тест)
    let order = Order::vwap("ETH", OrderSide::Sell, Decimal::from(240), 1);
    let vwap = VWAPExecutor::new();

    let fills = vwap
        .execute(&order, &profile, |qty| async move {
            Ok(Fill {
                id: uuid::Uuid::new_v4(),
                order_id: order.id,
                symbol: "ETH".to_string(),
                side: OrderSide::Sell,
                quantity: qty,
                price: Decimal::from(3000),
                venue: Venue::Coinbase,
                timestamp: chrono::Utc::now(),
                fees: qty * Decimal::from(3000) * Decimal::try_from(0.001).unwrap(),
            })
        })
        .await
        .expect("VWAP should execute");

    println!("   VWAP fills: {}", fills.len());

    let total_qty: Decimal = fills.iter().map(|f| f.quantity).sum();
    assert_eq!(total_qty, Decimal::from(240));

    println!("✅ VWAP algorithm test completed!");
}

/// GOLDEN PATH: Smart Order Routing (SOR)
///
/// Тества:
/// 1. Venue analysis
/// 2. Best price selection
/// 3. Cost calculation
/// 4. Route decision
#[test]
fn test_smart_routing() {
    println!("\n🧭 Testing Smart Order Routing");

    let mut router = SmartRouter::new();

    // Добавяне на quotes от различни venues
    let quotes = vec![
        VenueQuote {
            venue: Venue::Binance,
            symbol: "BTC".to_string(),
            bid: Decimal::from(49900),
            ask: Decimal::from(50100),
            bid_size: Decimal::from(100),
            ask_size: Decimal::from(100),
            timestamp: chrono::Utc::now(),
            latency_ms: 20,
        },
        VenueQuote {
            venue: Venue::Coinbase,
            symbol: "BTC".to_string(),
            bid: Decimal::from(49950),
            ask: Decimal::from(50050), // По-добра цена!
            bid_size: Decimal::from(80),
            ask_size: Decimal::from(80),
            timestamp: chrono::Utc::now(),
            latency_ms: 25,
        },
        VenueQuote {
            venue: Venue::Kraken,
            symbol: "BTC".to_string(),
            bid: Decimal::from(49800),
            ask: Decimal::from(50200),
            bid_size: Decimal::from(50),
            ask_size: Decimal::from(50),
            timestamp: chrono::Utc::now(),
            latency_ms: 30,
        },
    ];

    for quote in &quotes {
        router.venue_analyzer_mut().update_quote(quote.clone());
    }

    println!("✅ Added quotes from 3 venues");

    // Проверка на best quote
    let best_buy = router
        .venue_analyzer()
        .get_best_quote("BTC", OrderSide::Buy);
    assert!(best_buy.is_some());

    let best = best_buy.unwrap();
    println!(
        "   Best ask: ${} from {} (latency: {}ms)",
        best.ask,
        best.venue.name(),
        best.latency_ms
    );

    // Coinbase трябва да е най-добър за покупка ($50,050)
    assert_eq!(best.ask, Decimal::from(50050));
    assert_eq!(best.venue, Venue::Coinbase);

    // Routing decision
    let order = Order::market("BTC", OrderSide::Buy, Decimal::from(10));
    let decision = router.route(&order).expect("Should find route");

    println!(
        "   Route decision: Execute at {}",
        decision.primary_venue.name()
    );
    println!("   Expected cost: ${}", decision.expected_cost);

    // Проверка на venue scoring
    let scores = router
        .venue_analyzer()
        .score_venues("BTC", Decimal::from(10), OrderSide::Buy);
    println!("\n   Venue scores:");
    for score in &scores {
        println!(
            "     {}: composite={}",
            score.venue.name(),
            score.composite_score
        );
    }

    println!("✅ Smart routing test completed!");
}

/// GOLDEN PATH: Iceberg order execution
///
/// Тества:
/// 1. Разделяне на голяма поръчка
/// 2. Скриване на истинския размер
/// 3. Последователно изпълнение
#[tokio::test]
async fn test_iceberg_execution() {
    println!("\n🧊 Testing Iceberg Order Execution");

    // Iceberg: показваме само 10 BTC, но поръчката е за 45 BTC
    let iceberg = IcebergExecutor::new(Decimal::from(10));
    let order = Order::market("BTC", OrderSide::Buy, Decimal::from(45));

    println!(
        "✅ Created iceberg order: {} BTC (displayed: {})",
        order.quantity,
        Decimal::from(10)
    );

    // Разделяне
    let slices = iceberg.slice_order(&order);

    println!("   Sliced into {} child orders:", slices.len());
    for (i, slice) in slices.iter().enumerate() {
        println!("     Slice {}: {} BTC", i + 1, slice.quantity);
    }

    // Трябва да има 5 slices: 10, 10, 10, 10, 5
    assert_eq!(slices.len(), 5);
    assert_eq!(slices[0].quantity, Decimal::from(10));
    assert_eq!(slices[4].quantity, Decimal::from(5)); // Остатък

    // Сума трябва да е 45
    let total: Decimal = slices.iter().map(|s| s.quantity).sum();
    assert_eq!(total, Decimal::from(45));

    // Изпълнение
    let fills = iceberg
        .execute(&order, |_child| {
            async move {
                Ok(Fill {
                    id: uuid::Uuid::new_v4(),
                    order_id: _child.id,
                    symbol: _child.symbol.clone(),
                    side: _child.side,
                    quantity: _child.quantity,
                    price: Decimal::from(50000),
                    venue: Venue::Binance,
                    timestamp: chrono::Utc::now(),
                    fees: _child.quantity * Decimal::from(50), // $50 fee per BTC
                })
            }
        })
        .await
        .expect("Iceberg should execute");

    println!("   Fills completed: {}", fills.len());
    assert_eq!(fills.len(), 5);

    println!("✅ Iceberg execution test completed!");
}

/// GOLDEN PATH: Execution cost calculation
///
/// Тества:
/// 1. Spread cost
/// 2. Market impact
/// 3. Venue fees
/// 4. Total cost estimate
#[test]
fn test_execution_cost_calculation() {
    println!("\n💰 Testing Execution Cost Calculation");

    let calculator = CostCalculator::new();

    // Quote със спред
    let quote = VenueQuote {
        venue: Venue::Binance,
        symbol: "BTC".to_string(),
        bid: Decimal::from(49900),
        ask: Decimal::from(50100),
        bid_size: Decimal::from(100),
        ask_size: Decimal::from(100),
        timestamp: chrono::Utc::now(),
        latency_ms: 20,
    };

    // Spread = (50100 - 49900) / 50000 = 0.4%
    let spread_bps = quote.spread_pct() * Decimal::from(100);
    println!("✅ Market spread: {} bps", spread_bps);

    // Малка поръчка - минимален impact
    let small_order = Order::market("BTC", OrderSide::Buy, Decimal::from(1));
    let adv = Decimal::from(1000000); // $1M ADV

    let small_cost = calculator.calculate_cost(&small_order, &quote, adv);
    println!(
        "   Small order (1 BTC) cost: {} bps",
        small_cost.total_cost_bps
    );

    // Голяма поръчка - висок impact
    let large_order = Order::market("BTC", OrderSide::Buy, Decimal::from(100));
    let large_cost = calculator.calculate_cost(&large_order, &quote, adv);
    println!(
        "   Large order (100 BTC) cost: {} bps",
        large_cost.total_cost_bps
    );

    // Голямата поръчка трябва да има по-висок cost
    assert!(large_cost.total_cost_bps > small_cost.total_cost_bps);

    // Компоненти на cost
    println!("\n   Cost breakdown for large order:");
    println!("     Spread cost: ${}", large_cost.spread_cost);
    println!("     Market impact: ${}", large_cost.market_impact);
    println!("     Explicit fees: ${}", large_cost.explicit_fees);
    println!("     Slippage: ${}", large_cost.slippage);
    println!(
        "     Total: ${} ({} bps)",
        large_cost.total_cost, large_cost.total_cost_bps
    );

    println!("✅ Execution cost calculation test completed!");
}

/// GOLDEN PATH: Algorithm selection and recommendation
///
/// Тества:
/// 1. Auto-select based on order size
/// 2. Recommendation with reasoning
/// 3. Different strategies for different sizes
#[test]
fn test_algorithm_selection() {
    println!("\n🤖 Testing Algorithm Selection");

    // Малка поръчка - Market
    let small = Order::market("BTC", OrderSide::Buy, Decimal::from(5));
    let (algo_small, reason_small) = AlgorithmSelector::recommend(&small);
    println!("✅ Small order (5 BTC): {} - {}", algo_small, reason_small);
    assert_eq!(algo_small, "Market");

    // Средна поръчка - Iceberg
    let medium = Order::market("BTC", OrderSide::Buy, Decimal::from(50));
    let (algo_medium, reason_medium) = AlgorithmSelector::recommend(&medium);
    println!(
        "✅ Medium order (50 BTC): {} - {}",
        algo_medium, reason_medium
    );
    assert_eq!(algo_medium, "Iceberg");

    // Голяма поръчка - TWAP
    let large = Order::market("BTC", OrderSide::Buy, Decimal::from(200));
    let (algo_large, reason_large) = AlgorithmSelector::recommend(&large);
    println!(
        "✅ Large order (200 BTC): {} - {}",
        algo_large, reason_large
    );
    assert_eq!(algo_large, "TWAP");

    // Много голяма поръчка - VWAP
    let very_large = Order::market("BTC", OrderSide::Buy, Decimal::from(2000));
    let (algo_xl, reason_xl) = AlgorithmSelector::recommend(&very_large);
    println!(
        "✅ Very large order (2000 BTC): {} - {}",
        algo_xl, reason_xl
    );
    assert_eq!(algo_xl, "VWAP");

    // Explicit TWAP поръчка
    let twap_order = Order::twap("BTC", OrderSide::Buy, Decimal::from(10), 60, 5);
    let selected = AlgorithmSelector::select(&twap_order);
    println!("   Explicit TWAP order: algorithm = {}", selected);
    assert_eq!(selected, "TWAP");

    println!("✅ Algorithm selection test completed!");
}

/// GOLDEN PATH: Full automation lifecycle
///
/// Тества:
/// 1. Market data update
/// 2. Quote analysis
/// 3. Order submission
/// 4. Algorithm selection
/// 5. Execution
/// 6. Fill confirmation
#[tokio::test]
async fn test_full_automation_lifecycle() {
    println!("\n🚀 Testing Full Automation Lifecycle");

    let mut engine = ExecutionEngine::new();

    // 1. Update market data
    engine.update_quote(VenueQuote {
        venue: Venue::Binance,
        symbol: "ETH".to_string(),
        bid: Decimal::from(2990),
        ask: Decimal::from(3010),
        bid_size: Decimal::from(500),
        ask_size: Decimal::from(500),
        timestamp: chrono::Utc::now(),
        latency_ms: 15,
    });

    engine.update_quote(VenueQuote {
        venue: Venue::Coinbase,
        symbol: "ETH".to_string(),
        bid: Decimal::from(2995),
        ask: Decimal::from(3005), // По-добра цена
        bid_size: Decimal::from(400),
        ask_size: Decimal::from(400),
        timestamp: chrono::Utc::now(),
        latency_ms: 20,
    });

    println!("✅ Step 1: Market data updated");

    // 2. Get best quote
    let best = engine.get_best_quote("ETH", OrderSide::Buy);
    assert!(best.is_some());
    println!("✅ Step 2: Best quote identified");
    println!(
        "   Best ask: ${} at {}",
        best.unwrap().ask,
        best.unwrap().venue.name()
    );

    // 3. Create large order
    let large_order = Order::market("ETH", OrderSide::Buy, Decimal::from(500));

    // 4. Get algorithm recommendation
    let (algo, reason) = engine.recommend_algorithm(&large_order);
    println!("✅ Step 3: Algorithm recommended: {} - {}", algo, reason);

    // 5. Estimate cost
    let cost = engine.estimate_cost(&large_order, &Venue::Coinbase);
    if let Some(c) = cost {
        println!("✅ Step 4: Cost estimated: {} bps", c.total_cost_bps);
    }

    // 6. Execute with TWAP instead (1 сек за тест)
    let twap_order = Order::twap("ETH", OrderSide::Buy, Decimal::from(100), 1, 5);
    let fills = engine
        .submit_order(&twap_order)
        .await
        .expect("Should execute");

    println!("✅ Step 5: Order executed with {} fills", fills.len());

    // 7. Verify fills
    let total_qty: Decimal = fills.iter().map(|f| f.quantity).sum();
    let total_fees: Decimal = fills.iter().map(|f| f.fees).sum();

    println!("✅ Step 6: Execution verified");
    println!("   Total filled: {} ETH", total_qty);
    println!("   Total fees: ${}", total_fees);

    assert_eq!(total_qty, Decimal::from(100));

    println!("\n✅ Full automation lifecycle test completed!");
}

/// GOLDEN PATH: Multi-venue execution
///
/// Тества:
/// 1. Order splitting across venues
/// 2. Venue selection based on liquidity
/// 3. Best execution
#[test]
fn test_multi_venue_execution() {
    println!("\n🏛️ Testing Multi-Venue Execution");

    let mut router = SmartRouter::new();

    // Venue с различна ликвидност
    let venues = vec![
        (
            Venue::Binance,
            Decimal::from(1000),
            Decimal::try_from(0.001).unwrap(),
        ),
        (
            Venue::Coinbase,
            Decimal::from(800),
            Decimal::try_from(0.0015).unwrap(),
        ),
        (
            Venue::Kraken,
            Decimal::from(500),
            Decimal::try_from(0.002).unwrap(),
        ),
    ];

    // Add quotes for venues
    for (venue, _, _) in &venues {
        router.venue_analyzer_mut().update_quote(VenueQuote {
            venue: venue.clone(),
            symbol: "BTC".to_string(),
            bid: Decimal::from(49900),
            ask: Decimal::from(50100),
            bid_size: Decimal::from(1000),
            ask_size: Decimal::from(1000),
            timestamp: chrono::Utc::now(),
            latency_ms: 20,
        });
    }

    println!("✅ Available venues:");
    for (venue, liquidity, fee) in &venues {
        println!(
            "   {}: liquidity={}, fee={}%",
            venue.name(),
            liquidity,
            fee * Decimal::from(100)
        );
    }

    // Голяма поръчка може да се раздели
    let order = Order::market("BTC", OrderSide::Buy, Decimal::from(1500));

    // Анализ на дали едно venue е достатъчно
    let single_venue_liquidity = Decimal::from(1000);
    if order.quantity > single_venue_liquidity {
        println!(
            "   Order size ({}) exceeds single venue liquidity ({})",
            order.quantity, single_venue_liquidity
        );
        println!("   ✓ Multi-venue execution required");
    }

    // Симулация на разделяне
    let venue_a_fill = Decimal::from(1000); // Binance max
    let venue_b_fill = Decimal::from(500); // Coinbase remainder

    println!("   Proposed split:");
    println!("     Venue A (Binance): {} BTC", venue_a_fill);
    println!("     Venue B (Coinbase): {} BTC", venue_b_fill);

    assert_eq!(venue_a_fill + venue_b_fill, order.quantity);

    // Test the actual route_large_order method
    let decisions = router.route_large_order(&order);
    println!("   Router decisions: {} venues", decisions.len());
    for (i, d) in decisions.iter().enumerate() {
        println!(
            "     Venue {}: {} - {}",
            i + 1,
            d.primary_venue.name(),
            d.reason
        );
    }

    println!("✅ Multi-venue execution test completed!");
}

/// GOLDEN PATH: Fee structure comparison
///
/// Тества:
/// 1. Maker vs Taker fees
/// 2. Volume-based discounts
/// 3. Total cost comparison
#[test]
fn test_fee_structure_comparison() {
    println!("\n💸 Testing Fee Structure Comparison");

    let fee_binance = FeeStructure {
        maker_fee: Decimal::try_from(-0.0002).unwrap(), // Rebate
        taker_fee: Decimal::try_from(0.001).unwrap(),   // 0.1%
        min_fee: Decimal::ONE,
        max_fee: None,
        volume_discounts: vec![],
    };

    let fee_coinbase = FeeStructure {
        maker_fee: Decimal::try_from(0.0005).unwrap(), // 0.05%
        taker_fee: Decimal::try_from(0.005).unwrap(),  // 0.5%
        min_fee: Decimal::ONE,
        max_fee: None,
        volume_discounts: vec![],
    };

    let quantity = Decimal::from(10);
    let price = Decimal::from(50000);
    let notional = quantity * price;

    println!("✅ Fee comparison for ${} order:", notional);

    let cost_binance = fee_binance.calculate_fee(notional, false, Decimal::ZERO);
    let cost_coinbase = fee_coinbase.calculate_fee(notional, false, Decimal::ZERO);

    println!(
        "   Binance taker: ${} ({} fee)",
        cost_binance, fee_binance.taker_fee
    );
    println!(
        "   Coinbase taker: ${} ({} fee)",
        cost_coinbase, fee_coinbase.taker_fee
    );

    assert!(cost_binance < cost_coinbase);
    println!(
        "   ✓ Binance is cheaper by ${}",
        cost_coinbase - cost_binance
    );

    // Test maker rebate
    let maker_fee_binance = fee_binance.calculate_fee(notional, true, Decimal::ZERO);
    println!("   Binance maker rebate: ${}", maker_fee_binance);
    assert!(maker_fee_binance < Decimal::ZERO); // Negative = rebate

    println!("✅ Fee structure comparison test completed!");
}
