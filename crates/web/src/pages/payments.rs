//! Simulated NGN payments page – Milestone 5 (Presentation-ready testnet demo).
//!
//! Everything here is clearly labelled as mock / testnet-only. No real money,
//! no real payment provider, and no real NGN are ever involved.

use dioxus::prelude::*;

use crate::{
    components::{
        AddressQrCode, ArrowLeftIcon, CheckIcon, ExternalLinkIcon, NairaIcon, RefreshIcon,
        SendReceiveIcon, ShieldIcon, WarningIcon,
    },
    routes::Route,
    wallet::{format_satoshis, format_sats, validate_recipient},
};

// ---------------------------------------------------------------------------
// Mock exchange-rate data
// ---------------------------------------------------------------------------

/// Simulated BTC/NGN mid-market rate (as of a recent snapshot; not live).
const MOCK_BTC_NGN_RATE: f64 = 165_000_000.0; // ₦165,000,000 per BTC
/// Simulated provider fee percentage.
const PROVIDER_FEE_PCT: f64 = 1.5;
/// Simulated network fee in satoshis (rough estimate for a standard P2WPKH tx).
const NETWORK_FEE_SAT: u64 = 1_500;
/// Default testnet address used for demo settlements.
const DEMO_TESTNET_ADDRESS: &str = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";

// ---------------------------------------------------------------------------
// Local state types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaymentDirection {
    /// Pay simulated NGN -> Receive testnet BTC
    BuyBtc,
    /// Send testnet BTC -> Receive simulated NGN payout
    SellBtc,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputUnit {
    Btc,
    Ngn,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaymentStep {
    Quote,
    Review,
    Processing,
    Success,
}

#[derive(Clone, PartialEq)]
struct SimulatedOrder {
    id: String,
    direction: PaymentDirection,
    btc_sats: u64,
    ngn_total: f64,
    target_detail: String,
    txid: String,
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn Payments() -> Element {
    let mut direction = use_signal(|| PaymentDirection::BuyBtc);
    let mut input_unit = use_signal(|| InputUnit::Ngn);
    let mut btc_input = use_signal(|| "0.00050000".to_owned());
    let mut ngn_input = use_signal(|| "82500".to_owned());
    let mut recipient_address = use_signal(|| {
        crate::wallet_persistence::active_wallet_primary_address()
            .unwrap_or_else(|| DEMO_TESTNET_ADDRESS.to_owned())
    });
    let mut bank_name = use_signal(|| "Guaranty Trust Bank (Simulated)".to_owned());
    let mut account_number = use_signal(|| "0123456789".to_owned());
    let mut step = use_signal(|| PaymentStep::Quote);
    let mut active_order = use_signal(|| None::<SimulatedOrder>);
    let mut recent_orders = use_signal(Vec::<SimulatedOrder>::new);
    let mut copied_ref = use_signal(|| false);

    // Derived quote values
    let btc_amount_sat = parse_btc_sats(&btc_input.read());
    let base_ngn = btc_amount_sat.map(sats_to_ngn);
    let network_fee_ngn = sats_to_ngn(NETWORK_FEE_SAT);
    let provider_fee_ngn = base_ngn
        .unwrap_or(0.0)
        .mul_add(PROVIDER_FEE_PCT / 100.0, 0.0);

    let total_ngn = match direction() {
        PaymentDirection::BuyBtc => {
            base_ngn.unwrap_or(0.0).max(0.0) + network_fee_ngn + provider_fee_ngn
        }
        PaymentDirection::SellBtc => {
            (base_ngn.unwrap_or(0.0) - network_fee_ngn - provider_fee_ngn).max(0.0)
        }
    };

    let address_valid = validate_recipient(&recipient_address.read()).is_ok();
    let bank_valid = account_number.read().trim().len() == 10
        && account_number
            .read()
            .trim()
            .chars()
            .all(|c| c.is_ascii_digit());

    let can_review = btc_amount_sat.is_some()
        && match direction() {
            PaymentDirection::BuyBtc => address_valid,
            PaymentDirection::SellBtc => bank_valid,
        };

    let rate_display = format_ngn(MOCK_BTC_NGN_RATE);
    let btc_sat_display = btc_amount_sat
        .map(|s| format!("{} ({})", format_sats(s), format_satoshis(s)))
        .unwrap_or_else(|| "—".to_owned());
    let base_ngn_display = base_ngn.map(format_ngn).unwrap_or_else(|| "—".to_owned());
    let total_ngn_display = if btc_amount_sat.is_some() {
        format_ngn(total_ngn)
    } else {
        "—".to_owned()
    };
    let provider_fee_display = if btc_amount_sat.is_some() {
        format_ngn(provider_fee_ngn)
    } else {
        "—".to_owned()
    };
    let network_fee_display = format_ngn(network_fee_ngn);

    // Preset handler
    let mut apply_ngn_preset = move |ngn_val: f64| {
        input_unit.set(InputUnit::Ngn);
        ngn_input.set(format!("{}", ngn_val as u64));
        let sats = ((ngn_val / MOCK_BTC_NGN_RATE) * 100_000_000.0).round() as u64;
        let whole = sats / 100_000_000;
        let frac = sats % 100_000_000;
        btc_input.set(format!("{whole}.{frac:08}"));
    };

    let mut apply_btc_preset = move |sats: u64| {
        input_unit.set(InputUnit::Btc);
        let whole = sats / 100_000_000;
        let frac = sats % 100_000_000;
        btc_input.set(format!("{whole}.{frac:08}"));
        let ngn = sats_to_ngn(sats);
        ngn_input.set(format!("{}", ngn.round() as u64));
    };

    // Simulate payment execution and credit/debit the active BitSabi wallet
    let do_simulate_payment = use_callback(move |()| {
        let Some(sats) = parse_btc_sats(&btc_input.read()) else {
            return;
        };
        step.set(PaymentStep::Processing);
        let current_dir = direction();
        let existing_count = crate::wallet_persistence::load_simulated_txs().len()
            + recent_orders.read().len()
            + 1;
        let order_id = format!("BSB-TEST-{existing_count:04}");
        let fake_txid = format!(
            "b175ab10{:08x}cafe0000deadbeef1234567890abcdef1234567890{:06x}",
            sats.wrapping_add(existing_count as u64),
            existing_count
        );
        let wallet_addr = crate::wallet_persistence::active_wallet_primary_address()
            .unwrap_or_else(|| recipient_address.read().trim().to_owned());
        let target = match current_dir {
            PaymentDirection::BuyBtc => recipient_address.read().trim().to_owned(),
            PaymentDirection::SellBtc => {
                format!("{} · {}", bank_name.read(), account_number.read().trim())
            }
        };
        let computed_base = sats_to_ngn(sats);
        let computed_fee = computed_base * (PROVIDER_FEE_PCT / 100.0);
        let computed_net = sats_to_ngn(NETWORK_FEE_SAT);
        let final_ngn = match current_dir {
            PaymentDirection::BuyBtc => computed_base + computed_fee + computed_net,
            PaymentDirection::SellBtc => (computed_base - computed_fee - computed_net).max(0.0),
        };

        let tx_for_wallet = crate::wallet_persistence::SimulatedWalletTx {
            txid: fake_txid.clone(),
            address: if current_dir == PaymentDirection::BuyBtc {
                target.clone()
            } else {
                wallet_addr
            },
            net_value_sat: if current_dir == PaymentDirection::BuyBtc {
                sats as i64
            } else {
                -(sats as i64)
            },
            fee_sat: NETWORK_FEE_SAT,
            timestamp: 1_761_830_400 + (existing_count as u64 * 120),
        };

        let order = SimulatedOrder {
            id: order_id,
            direction: current_dir,
            btc_sats: sats,
            ngn_total: final_ngn,
            target_detail: target,
            txid: fake_txid,
        };

        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::TimeoutFuture::new(900).await;
            crate::wallet_persistence::record_simulated_tx(tx_for_wallet);
            active_order.set(Some(order.clone()));
            recent_orders.write().insert(0, order);
            step.set(PaymentStep::Success);
        });
    });

    let reset_quote = use_callback(move |()| {
        step.set(PaymentStep::Quote);
        copied_ref.set(false);
    });

    rsx! {
        div { class: "page",
            // ----------------------------------------------------------------
            // Page header
            // ----------------------------------------------------------------
            header { class: "page-header",
                div { class: "page-header__meta",
                    p { class: "eyebrow", "NGN Payments · Testnet Sandbox" }
                    span { class: "pill pill--orange", "Simulated · Testnet only" }
                }
                h1 { "Bitcoin ⇄ Naira Quote & Settlement Demo" }
                p {
                    "Explore how buying and selling Bitcoin with NGN works in practice. Inspect exchange rates, provider fees, and Bitcoin network fees before simulating a testnet settlement."
                }
            }

            // ----------------------------------------------------------------
            // Persistent disclaimer banner
            // ----------------------------------------------------------------
            div { class: "notice notice--warning",
                WarningIcon {}
                div {
                    strong { "Simulation mode — no real money or bank transfers" }
                    p {
                        "Rates, bank details, and NGN balances on this screen are simulated for demonstration and testing. Only Bitcoin Testnet addresses are accepted."
                    }
                }
            }

            // ================================================================
            // STEP 1: Quote calculator
            // ================================================================
            if step() == PaymentStep::Quote {
                section { class: "section", aria_labelledby: "quote-heading",
                    div { class: "wallet-panel__header",
                        div {
                            h2 { id: "quote-heading", "1. Configure your quote" }
                            p { "Choose whether to buy or sell testnet Bitcoin and enter an amount." }
                        }
                        div { class: "wallet-tabs", role: "tablist",
                            button {
                                class: if direction() == PaymentDirection::BuyBtc {
                                    "wallet-tab wallet-tab--active"
                                } else {
                                    "wallet-tab"
                                },
                                r#type: "button",
                                onclick: move |_| direction.set(PaymentDirection::BuyBtc),
                                "Buy Testnet BTC (NGN → tBTC)"
                            }
                            button {
                                class: if direction() == PaymentDirection::SellBtc {
                                    "wallet-tab wallet-tab--active"
                                } else {
                                    "wallet-tab"
                                },
                                r#type: "button",
                                onclick: move |_| direction.set(PaymentDirection::SellBtc),
                                "Sell Testnet BTC (tBTC → NGN)"
                            }
                        }
                    }

                    // Quick amount presets for presentation
                    div { class: "payments-presets",
                        span { class: "payments-presets__label", "Quick demo amounts:" }
                        button {
                            class: "button button--secondary button--compact",
                            r#type: "button",
                            onclick: move |_| apply_ngn_preset(16_500.0),
                            "₦16,500 (10k sats)"
                        }
                        button {
                            class: "button button--secondary button--compact",
                            r#type: "button",
                            onclick: move |_| apply_ngn_preset(82_500.0),
                            "₦82,500 (50k sats)"
                        }
                        button {
                            class: "button button--secondary button--compact",
                            r#type: "button",
                            onclick: move |_| apply_ngn_preset(250_000.0),
                            "₦250,000"
                        }
                        button {
                            class: "button button--secondary button--compact",
                            r#type: "button",
                            onclick: move |_| apply_btc_preset(100_000),
                            "0.001 tBTC"
                        }
                        button {
                            class: "button button--secondary button--compact",
                            r#type: "button",
                            onclick: move |_| apply_btc_preset(500_000),
                            "0.005 tBTC"
                        }
                    }

                    div { class: "wallet-form-grid",
                        // --- NGN input ---
                        div { class: "wallet-field",
                            label { r#for: "pay-ngn-amount",
                                if direction() == PaymentDirection::BuyBtc {
                                    "Simulated NGN amount to convert"
                                } else {
                                    "Target NGN value before fees"
                                }
                            }
                            div { class: "payments-input-row",
                                input {
                                    id: "pay-ngn-amount",
                                    class: "wallet-input",
                                    r#type: "text",
                                    inputmode: "decimal",
                                    value: "{ngn_input}",
                                    placeholder: "82500",
                                    autocomplete: "off",
                                    oninput: move |ev| {
                                        let val = ev.value();
                                        input_unit.set(InputUnit::Ngn);
                                        ngn_input.set(val.clone());
                                        let cleaned: String = val.chars().filter(|c| *c != ',').collect();
                                        if let Ok(ngn) = cleaned.trim().parse::<f64>() {
                                            if ngn > 0.0 {
                                                let sats = ((ngn / MOCK_BTC_NGN_RATE) * 100_000_000.0).round() as u64;
                                                let whole = sats / 100_000_000;
                                                let frac = sats % 100_000_000;
                                                btc_input.set(format!("{whole}.{frac:08}"));
                                            } else {
                                                btc_input.set(String::new());
                                            }
                                        } else {
                                            btc_input.set(String::new());
                                        }
                                    },
                                }
                                span { class: "payments-input-unit", "₦ NGN" }
                            }
                            p { class: "wallet-field__help",
                                "Type any Naira amount — the testnet BTC amount updates automatically."
                            }
                        }

                        // --- BTC input ---
                        div { class: "wallet-field",
                            label { r#for: "pay-btc-amount",
                                if direction() == PaymentDirection::BuyBtc {
                                    "Testnet BTC you will receive"
                                } else {
                                    "Testnet BTC you are selling"
                                }
                            }
                            div { class: "payments-input-row",
                                input {
                                    id: "pay-btc-amount",
                                    class: "wallet-input",
                                    r#type: "text",
                                    inputmode: "decimal",
                                    value: "{btc_input}",
                                    placeholder: "0.00050000",
                                    autocomplete: "off",
                                    oninput: move |ev| {
                                        let val = ev.value();
                                        input_unit.set(InputUnit::Btc);
                                        btc_input.set(val.clone());
                                        if let Some(sats) = parse_btc_sats(&val) {
                                            let ngn = sats_to_ngn(sats);
                                            ngn_input.set(format!("{}", ngn.round() as u64));
                                        } else {
                                            ngn_input.set(String::new());
                                        }
                                    },
                                }
                                span { class: "payments-input-unit", "tBTC" }
                            }
                            p { class: "wallet-field__help",
                                if let Some(sats) = btc_amount_sat {
                                    "Exact satoshis: {format_satoshis(sats)}"
                                } else {
                                    "Up to 8 decimal places (minimum 1 satoshi)."
                                }
                            }
                        }
                    }

                    // Destination details depending on Buy vs Sell
                    div { class: "wallet-form",
                        if direction() == PaymentDirection::BuyBtc {
                            div { class: "wallet-field",
                                div { class: "wallet-field__meta",
                                    label { r#for: "pay-recipient-address", "Receiving Bitcoin testnet address" }
                                    button {
                                        class: "button button--secondary button--compact",
                                        r#type: "button",
                                        onclick: move |_| {
                                            let addr = crate::wallet_persistence::active_wallet_primary_address()
                                                .unwrap_or_else(|| DEMO_TESTNET_ADDRESS.to_owned());
                                            recipient_address.set(addr);
                                        },
                                        "Use my wallet address"
                                    }
                                }
                                input {
                                    id: "pay-recipient-address",
                                    class: "wallet-input",
                                    r#type: "text",
                                    value: "{recipient_address}",
                                    placeholder: "tb1q...",
                                    autocomplete: "off",
                                    spellcheck: false,
                                    oninput: move |ev| recipient_address.set(ev.value()),
                                }
                                if !recipient_address.read().trim().is_empty() && !address_valid {
                                    p { class: "wallet-error",
                                        "Enter a valid Bitcoin testnet address (e.g. starting with tb1q, m, n, or 2). Mainnet addresses are rejected."
                                    }
                                } else {
                                    p { class: "wallet-field__help",
                                        "Simulated testnet BTC purchases are credited directly to your BitSabi Wallet balance and history."
                                    }
                                }
                            }
                        } else {
                            div { class: "wallet-form-grid",
                                div { class: "wallet-field",
                                    label { r#for: "pay-bank-name", "Simulated NGN payout bank" }
                                    input {
                                        id: "pay-bank-name",
                                        class: "wallet-input",
                                        r#type: "text",
                                        value: "{bank_name}",
                                        oninput: move |ev| bank_name.set(ev.value()),
                                    }
                                }
                                div { class: "wallet-field",
                                    label { r#for: "pay-account-number", "Simulated 10-digit NUBAN account" }
                                    input {
                                        id: "pay-account-number",
                                        class: "wallet-input",
                                        r#type: "text",
                                        inputmode: "numeric",
                                        maxlength: 10,
                                        value: "{account_number}",
                                        placeholder: "0123456789",
                                        oninput: move |ev| account_number.set(ev.value()),
                                    }
                                    if !bank_valid {
                                        p { class: "wallet-error", "Enter a 10-digit numeric account number." }
                                    }
                                }
                            }
                        }
                    }

                    // --------------------------------------------------------
                    // Quote summary card
                    // --------------------------------------------------------
                    div { class: "payments-quote-card",
                        div { class: "payments-quote-card__header",
                            div {
                                h3 {
                                    if direction() == PaymentDirection::BuyBtc {
                                        "Simulated Buy Quote (On-ramp)"
                                    } else {
                                        "Simulated Sell Quote (Off-ramp)"
                                    }
                                }
                            }
                            span { class: "pill", "1 BTC = {rate_display} (Mock)" }
                        }
                        div { class: "wallet-review__rows",
                            div { class: "wallet-review__row",
                                span {
                                    if direction() == PaymentDirection::BuyBtc {
                                        "Testnet BTC to receive"
                                    } else {
                                        "Testnet BTC to sell"
                                    }
                                }
                                strong { "{btc_sat_display}" }
                            }
                            div { class: "wallet-review__row",
                                span { "Mid-market NGN value" }
                                strong { "{base_ngn_display}" }
                            }
                            div { class: "wallet-review__row",
                                span { "Bitcoin network fee (est.)" }
                                strong { "{network_fee_display} ({format_satoshis(NETWORK_FEE_SAT)})" }
                            }
                            div { class: "wallet-review__row",
                                span { "Provider fee ({PROVIDER_FEE_PCT}%)" }
                                strong { "{provider_fee_display}" }
                            }
                            div { class: "wallet-review__row wallet-review__row--total",
                                span {
                                    if direction() == PaymentDirection::BuyBtc {
                                        "Total simulated NGN you pay"
                                    } else {
                                        "Net simulated NGN payout you receive"
                                    }
                                }
                                strong { "{total_ngn_display}" }
                            }
                        }

                        div { class: "wallet-flow__actions",
                            Link {
                                class: "button button--secondary",
                                to: Route::Wallet {},
                                "Open Testnet Wallet"
                            }
                            button {
                                class: "button button--primary",
                                r#type: "button",
                                disabled: !can_review,
                                onclick: move |_| step.set(PaymentStep::Review),
                                if direction() == PaymentDirection::BuyBtc {
                                    "Continue to simulated payment"
                                } else {
                                    "Continue to simulated payout"
                                }
                            }
                        }
                    }
                }
            }

            // ================================================================
            // STEP 2: Review & Settlement Instructions
            // ================================================================
            if step() == PaymentStep::Review {
                section { class: "section",
                    div { class: "wallet-review",
                        div { class: "wallet-panel__header",
                            div {
                                h2 {
                                    if direction() == PaymentDirection::BuyBtc {
                                        "2. Simulated NGN Bank Transfer"
                                    } else {
                                        "2. Simulated Testnet BTC Deposit"
                                    }
                                }
                                p {
                                    if direction() == PaymentDirection::BuyBtc {
                                        "In production, you would transfer NGN to the virtual account below. Click 'Simulate Payment Confirmed' to complete the testnet demo."
                                    } else {
                                        "In production, you would send BTC to the settlement address below to trigger your NGN bank payout."
                                    }
                                }
                            }
                            span { class: "pill pill--orange", "Testnet Demo" }
                        }

                        if direction() == PaymentDirection::BuyBtc {
                            div { class: "payments-quote-card",
                                div { class: "payments-quote-card__header",
                                    h3 { "Simulated Virtual Bank Account" }
                                    span { class: "pill", "Do not send real money" }
                                }
                                div { class: "wallet-review__rows",
                                    div { class: "wallet-review__row",
                                        span { "Bank Name" }
                                        strong { "BitSabi Sandbox Bank (Test)" }
                                    }
                                    div { class: "wallet-review__row",
                                        span { "Account Number" }
                                        code { "9900112233" }
                                    }
                                    div { class: "wallet-review__row",
                                        span { "Exact Amount" }
                                        strong { "{total_ngn_display}" }
                                    }
                                    div { class: "wallet-review__row",
                                        span { "Recipient Testnet Address" }
                                        code { "{recipient_address}" }
                                    }
                                    div { class: "wallet-review__row",
                                        span { "Testnet BTC to Deliver" }
                                        strong { "{btc_sat_display}" }
                                    }
                                }
                            }
                        } else {
                            div { class: "wallet-receive-grid",
                                AddressQrCode { value: DEMO_TESTNET_ADDRESS.to_owned() }
                                div { class: "wallet-receive__details",
                                    span { class: "wallet-card__label", "Simulated Settlement Deposit Address (Testnet)" }
                                    code { class: "wallet-address", "{DEMO_TESTNET_ADDRESS}" }
                                    div { class: "wallet-review__rows",
                                        div { class: "wallet-review__row",
                                            span { "BTC to Deposit" }
                                            strong { "{btc_sat_display}" }
                                        }
                                        div { class: "wallet-review__row",
                                            span { "Payout Bank" }
                                            strong { "{bank_name} · {account_number}" }
                                        }
                                        div { class: "wallet-review__row wallet-review__row--total",
                                            span { "Simulated NGN Payout" }
                                            strong { "{total_ngn_display}" }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "wallet-flow__actions",
                            button {
                                class: "button button--secondary",
                                r#type: "button",
                                onclick: move |_| step.set(PaymentStep::Quote),
                                ArrowLeftIcon {}
                                "Edit quote"
                            }
                            button {
                                class: "button button--primary",
                                r#type: "button",
                                onclick: move |_| do_simulate_payment.call(()),
                                CheckIcon {}
                                if direction() == PaymentDirection::BuyBtc {
                                    "Simulate NGN Transfer & Settle"
                                } else {
                                    "Simulate BTC Deposit & Payout"
                                }
                            }
                        }
                    }
                }
            }

            // ================================================================
            // STEP 3: Processing spinner
            // ================================================================
            if step() == PaymentStep::Processing {
                div { class: "wallet-loading", role: "status",
                    RefreshIcon {}
                    div {
                        strong { "Simulating settlement..." }
                        p { "Matching simulated NGN quote with Bitcoin Testnet settlement." }
                    }
                }
            }

            // ================================================================
            // STEP 4: Success receipt
            // ================================================================
            if step() == PaymentStep::Success {
                if let Some(order) = active_order.read().clone() {
                    div { class: "wallet-success", role: "status",
                        div { class: "wallet-success__icon", CheckIcon {} }
                        div {
                            div { class: "wallet-panel__header",
                                h2 {
                                    if order.direction == PaymentDirection::BuyBtc {
                                        "Simulated BTC Purchase Complete"
                                    } else {
                                        "Simulated NGN Payout Complete"
                                    }
                                }
                                span { class: "pill pill--orange", "Receipt #{order.id}" }
                            }
                            p {
                                "This receipt demonstrates how BitSabi presents transparent settlement details once a payment completes."
                            }

                            div { class: "wallet-review__rows",
                                div { class: "wallet-review__row",
                                    span { "Reference ID" }
                                    strong { "{order.id}" }
                                }
                                div { class: "wallet-review__row",
                                    span { "Type" }
                                    strong {
                                        if order.direction == PaymentDirection::BuyBtc {
                                            "Buy Testnet BTC (NGN → tBTC)"
                                        } else {
                                            "Sell Testnet BTC (tBTC → NGN)"
                                        }
                                    }
                                }
                                div { class: "wallet-review__row",
                                    span { "Bitcoin Amount" }
                                    strong { "{format_sats(order.btc_sats)} ({format_satoshis(order.btc_sats)})" }
                                }
                                div { class: "wallet-review__row",
                                    span {
                                        if order.direction == PaymentDirection::BuyBtc {
                                            "Simulated NGN Paid"
                                        } else {
                                            "Simulated NGN Paid Out"
                                        }
                                    }
                                    strong { "{format_ngn(order.ngn_total)}" }
                                }
                                div { class: "wallet-review__row",
                                    span {
                                        if order.direction == PaymentDirection::BuyBtc {
                                            "Delivered to Testnet Address"
                                        } else {
                                            "Settled to Bank Account"
                                        }
                                    }
                                    code { "{order.target_detail}" }
                                }
                                div { class: "wallet-review__row",
                                    span { "Simulated Settlement Reference" }
                                    code { class: "wallet-success__txid", "{order.txid}" }
                                }
                            }

                            if order.direction == PaymentDirection::BuyBtc {
                                a {
                                    class: "text-link",
                                    href: "https://mempool.space/testnet/address/{order.target_detail}",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    "Inspect recipient address on mempool.space (testnet)"
                                    ExternalLinkIcon {}
                                }
                            }

                            div { class: "wallet-flow__actions",
                                button {
                                    class: "button button--primary",
                                    r#type: "button",
                                    onclick: move |_| reset_quote.call(()),
                                    SendReceiveIcon {}
                                    "Create another quote"
                                }
                                Link {
                                    class: "button button--secondary",
                                    to: Route::Wallet {},
                                    "Go to Testnet Wallet"
                                }
                            }
                        }
                    }
                }
            }

            // ================================================================
            // Recent simulated orders in this session
            // ================================================================
            if !recent_orders.read().is_empty() {
                section { class: "section", aria_labelledby: "recent-orders-heading",
                    h2 { id: "recent-orders-heading", "Session simulated activity" }
                    div { class: "wallet-history",
                        for order in recent_orders.read().iter() {
                            article { key: "{order.id}", class: "wallet-tx wallet-tx--received",
                                div { class: "wallet-tx__icon", NairaIcon {} }
                                div { class: "wallet-tx__body",
                                    div { class: "wallet-tx__head",
                                        div {
                                            strong {
                                                if order.direction == PaymentDirection::BuyBtc {
                                                    "Bought Testnet BTC"
                                                } else {
                                                    "Sold Testnet BTC"
                                                }
                                            }
                                            span { class: "wallet-tx__status", "Simulated · {order.id}" }
                                        }
                                        div { class: "wallet-tx__amounts",
                                            strong { class: "wallet-tx__amount", "{format_sats(order.btc_sats)}" }
                                            span { class: "wallet-tx__amount-sats", "{format_ngn(order.ngn_total)}" }
                                        }
                                    }
                                    div { class: "wallet-tx__meta",
                                        span { "Destination: {order.target_detail}" }
                                        code { class: "wallet-tx__txid", "Ref: {order.txid}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ----------------------------------------------------------------
            // How it works & safety
            // ----------------------------------------------------------------
            section { class: "section", aria_labelledby: "payment-steps-heading",
                h2 { id: "payment-steps-heading", "How real NGN settlement will work" }
                ol { class: "step-list",
                    li {
                        span { class: "step-list__number", "1" }
                        div {
                            h3 { "Transparent quote upfront" }
                            p { "Enter a BTC target or NGN budget. Both the provider spread and the Bitcoin network fee are itemized clearly." }
                        }
                    }
                    li {
                        span { class: "step-list__number", "2" }
                        div {
                            h3 { "Self-custody address verification" }
                            p { "When buying Bitcoin, funds are delivered straight to your BitSabi self-custody wallet address." }
                        }
                    }
                    li {
                        span { class: "step-list__number", "3" }
                        div {
                            h3 { "Instant bank transfer or on-chain confirmation" }
                            p { "A licensed Nigerian payment partner verifies the NGN bank transfer or BTC deposit and settles automatically." }
                        }
                    }
                }
            }

            aside { class: "panel panel--soft",
                ShieldIcon {}
                div {
                    h2 { "Before real payments are enabled" }
                    p {
                        "Real NGN payments require a licensed payment provider, verified webhooks, settlement accounting, and compliance review. BitSabi keeps this testnet sandbox 100% free of real money risk."
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a BTC text input into satoshis. Returns `None` for invalid input.
fn parse_btc_sats(input: &str) -> Option<u64> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let mut parts = input.split('.');
    let whole: u64 = parts.next()?.parse().ok()?;
    let sats = whole.checked_mul(100_000_000)?;
    let sats = if let Some(frac) = parts.next() {
        if parts.next().is_some() || frac.is_empty() || frac.len() > 8 {
            return None;
        }
        if !frac.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let frac_val: u64 = frac.parse().ok()?;
        let scale = 10_u64.pow((8 - frac.len()) as u32);
        sats.checked_add(frac_val.checked_mul(scale)?)?
    } else {
        sats
    };
    if sats == 0 {
        return None;
    }
    Some(sats)
}

/// Convert satoshis to a simulated NGN value.
fn sats_to_ngn(sats: u64) -> f64 {
    (sats as f64) / 100_000_000.0 * MOCK_BTC_NGN_RATE
}

/// Format a naira amount with thousand separators and the ₦ symbol.
fn format_ngn(ngn: f64) -> String {
    let whole = ngn.floor() as u64;
    let cents = ((ngn - whole as f64) * 100.0).round() as u64;
    let (whole, cents) = if cents >= 100 {
        (whole + 1, 0)
    } else {
        (whole, cents)
    };
    let digits = whole.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 5);
    grouped.push('₦');
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(character);
    }
    grouped.push_str(&format!(".{cents:02}"));
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_btc_inputs_to_sats() {
        assert_eq!(parse_btc_sats("0.0005"), Some(50_000));
        assert_eq!(parse_btc_sats("1"), Some(100_000_000));
        assert_eq!(parse_btc_sats("0"), None);
        assert_eq!(parse_btc_sats("abc"), None);
    }

    #[test]
    fn formats_ngn_with_commas_and_kobo() {
        assert_eq!(format_ngn(82_500.0), "₦82,500.00");
        assert_eq!(format_ngn(165_000_000.0), "₦165,000,000.00");
    }
}
