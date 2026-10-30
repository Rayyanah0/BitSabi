//! Testnet wallet creation, import, balance, receive, send, and history.

use dioxus::prelude::*;
use zeroize::Zeroizing;

use bitsabi_shared::{FeeEstimates, WalletSnapshot};

use crate::{
    api,
    components::{
        AddressQrCode, ArrowLeftIcon, CheckIcon, ClockIcon, CopyIcon, ExternalLinkIcon,
        ImportIcon, PrimaryButton, RefreshIcon, SecondaryButton, SendReceiveIcon, ShieldIcon,
        WalletIcon, WarningIcon,
    },
    wallet::{
        format_sats, format_signed_sats, parse_btc_amount, plan_send, sign_send_plan,
        SendPlan, SignedTransaction, WalletSession,
    },
    wallet_data::{HistoryTransaction, TransactionDirection, WalletSummary},
};

const HISTORY_PAGE_SIZE: usize = 8;
const OVERVIEW_TX_LIMIT: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum WalletStage {
    Landing,
    Create,
    Import,
    Ready,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WalletSource {
    Created,
    Imported,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WalletPanel {
    Overview,
    Receive,
    Send,
    History,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyState {
    Idle,
    Copied,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FeeChoice {
    Fastest,
    Normal,
    Economy,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BroadcastState {
    Idle,
    Broadcasting,
    Success {
        txid: String,
        fee_sat: u64,
        vsize: u64,
    },
    Failed(String),
}

#[component]
pub fn Wallet() -> Element {
    let mut stage = use_signal(|| WalletStage::Landing);
    let mut created_wallet = use_signal(|| None::<crate::wallet::CreatedWallet>);
    let mut import_phrase = use_signal(|| Zeroizing::new(String::new()));
    let mut backup_confirmed = use_signal(|| false);
    let mut session = use_signal(|| None::<WalletSession>);
    let mut source = use_signal(|| None::<WalletSource>);
    let mut wallet_error = use_signal(|| None::<String>);
    let mut copy_state = use_signal(|| CopyState::Idle);
    let mut copied_address = use_signal(|| None::<String>);
    let mut panel = use_signal(|| WalletPanel::Overview);
    let mut snapshot = use_signal(|| None::<WalletSnapshot>);
    let mut summary = use_signal(|| None::<WalletSummary>);
    let mut loading = use_signal(|| false);
    let mut refresh_error = use_signal(|| None::<String>);
    let mut refresh_id = use_signal(|| 0_u64);
    let mut send_address = use_signal(String::new);
    let mut send_amount = use_signal(String::new);
    let mut fee_choice = use_signal(|| FeeChoice::Normal);
    let mut custom_fee_rate = use_signal(String::new);
    let mut send_error = use_signal(|| None::<String>);
    let mut send_plan = use_signal(|| None::<SendPlan>);
    let mut send_signed = use_signal(|| None::<SignedTransaction>);
    let mut broadcast_state = use_signal(|| BroadcastState::Idle);
    let mut sending = use_signal(|| false);
    let mut visible_tx_count = use_signal(|| HISTORY_PAGE_SIZE);

    let import_word_count = import_phrase.read().split_whitespace().count();

    let reset_to_landing = use_callback(move |()| {
        refresh_id.set(refresh_id().wrapping_add(1));
        session.set(None);
        created_wallet.set(None);
        import_phrase.set(Zeroizing::new(String::new()));
        backup_confirmed.set(false);
        source.set(None);
        wallet_error.set(None);
        copy_state.set(CopyState::Idle);
        copied_address.set(None);
        panel.set(WalletPanel::Overview);
        snapshot.set(None);
        summary.set(None);
        loading.set(false);
        refresh_error.set(None);
        send_address.set(String::new());
        send_amount.set(String::new());
        fee_choice.set(FeeChoice::Normal);
        custom_fee_rate.set(String::new());
        send_error.set(None);
        send_plan.set(None);
        send_signed.set(None);
        broadcast_state.set(BroadcastState::Idle);
        sending.set(false);
        visible_tx_count.set(HISTORY_PAGE_SIZE);
        stage.set(WalletStage::Landing);
    });

    let begin_wallet = use_callback(move |(new_session, new_source): (WalletSession, WalletSource)| {
        let addresses = new_session.scan_addresses();
        session.set(Some(new_session));
        source.set(Some(new_source));
        wallet_error.set(None);
        copy_state.set(CopyState::Idle);
        copied_address.set(None);
        panel.set(WalletPanel::Overview);
        snapshot.set(None);
        summary.set(None);
        refresh_error.set(None);
        send_address.set(String::new());
        send_amount.set(String::new());
        fee_choice.set(FeeChoice::Normal);
        custom_fee_rate.set(String::new());
        send_error.set(None);
        send_plan.set(None);
        send_signed.set(None);
        broadcast_state.set(BroadcastState::Idle);
        sending.set(false);
        visible_tx_count.set(HISTORY_PAGE_SIZE);
        stage.set(WalletStage::Ready);
        request_wallet_refresh(
            addresses,
            true,
            snapshot,
            summary,
            loading,
            refresh_error,
            refresh_id,
        );
    });

    let start_create = use_callback(move |()| {
        reset_to_landing.call(());

        match crate::wallet::create_testnet_wallet() {
            Ok(wallet) => {
                created_wallet.set(Some(wallet));
                wallet_error.set(None);
                stage.set(WalletStage::Create);
            }
            Err(error) => {
                wallet_error.set(Some(error.to_string()));
                stage.set(WalletStage::Landing);
            }
        }
    });

    let start_import = use_callback(move |()| {
        reset_to_landing.call(());
        stage.set(WalletStage::Import);
    });

    let import_wallet = use_callback(move |()| {
        let result = {
            let phrase = import_phrase.read();
            if phrase.trim().is_empty() {
                Err("Enter a 12- or 24-word recovery phrase.".to_owned())
            } else {
                WalletSession::from_phrase(phrase.as_str())
                    .map_err(|error| error.to_string())
            }
        };

        match result {
            Ok(new_session) => {
                import_phrase.set(Zeroizing::new(String::new()));
                begin_wallet.call((new_session, WalletSource::Imported));
            }
            Err(message) => wallet_error.set(Some(message)),
        }
    });

    let confirm_backup = use_callback(move |()| {
        if !backup_confirmed() {
            return;
        }

        let result = {
            let wallet = created_wallet.read();
            let Some(wallet) = wallet.as_ref() else {
                return;
            };
            WalletSession::from_phrase(wallet.phrase())
        };

        match result {
            Ok(new_session) => {
                created_wallet.set(None);
                backup_confirmed.set(false);
                begin_wallet.call((new_session, WalletSource::Created));
            }
            Err(error) => {
                wallet_error.set(Some(error.to_string()));
            }
        }
    });

    let manual_refresh = use_callback(move |()| {
        let addresses = session
            .read()
            .as_ref()
            .map(WalletSession::scan_addresses)
            .unwrap_or_default();
        if addresses.is_empty() {
            return;
        }
        request_wallet_refresh(
            addresses,
            true,
            snapshot,
            summary,
            loading,
            refresh_error,
            refresh_id,
        );
    });

    let generate_address = use_callback(move |()| {
        let result = {
            let mut wallet = session.write();
            match wallet.as_mut() {
                Some(wallet) => match wallet.generate_receive_address() {
                    Ok(address) => {
                        let address = address.address().to_owned();
                        Ok((address, wallet.scan_addresses()))
                    }
                    Err(error) => Err(error.to_string()),
                },
                None => Err("The wallet session is not available. Start over.".to_owned()),
            }
        };

        match result {
            Ok((_address, addresses)) => {
                copy_state.set(CopyState::Idle);
                copied_address.set(None);
                request_wallet_refresh(
                    addresses,
                    false,
                    snapshot,
                    summary,
                    loading,
                    refresh_error,
                    refresh_id,
                );
            }
            Err(message) => wallet_error.set(Some(message)),
        }
    });

    let review_send = use_callback(move |()| {
        send_error.set(None);
        send_plan.set(None);
        send_signed.set(None);
        broadcast_state.set(BroadcastState::Idle);

        let recipient = send_address.read().trim().to_owned();
        let amount_text = send_amount.read().trim().to_owned();
        let fee_choice_value = fee_choice();
        let custom_fee = custom_fee_rate.read().trim().to_owned();
        let Some(fees) = snapshot.read().as_ref().map(|snapshot| snapshot.fees) else {
            send_error.set(Some(
                "Load the wallet balance and fee estimates before sending.".to_owned(),
            ));
            return;
        };

        let fee_rate = match fee_rate_for_choice(fee_choice_value, &custom_fee, fees) {
            Ok(rate) => rate,
            Err(message) => {
                send_error.set(Some(message));
                return;
            }
        };
        let amount_sat = match parse_btc_amount(&amount_text) {
            Ok(amount) => amount,
            Err(error) => {
                send_error.set(Some(error.to_string()));
                return;
            }
        };
        let spendable = summary
            .read()
            .as_ref()
            .map(WalletSummary::spendable_utxos)
            .unwrap_or_default();

        let result = {
            let wallet = session.read();
            match wallet.as_ref() {
                Some(wallet) => {
                    plan_send(wallet, &spendable, &recipient, amount_sat, fee_rate)
                        .map_err(|error| error.to_string())
                }
                None => Err("The wallet session is not available. Start over.".to_owned()),
            }
        };

        match result {
            Ok(plan) => {
                send_plan.set(Some(plan));
                send_error.set(None);
            }
            Err(message) => send_error.set(Some(message)),
        }
    });

    let edit_send = use_callback(move |()| {
        send_plan.set(None);
        send_signed.set(None);
        send_error.set(None);
        broadcast_state.set(BroadcastState::Idle);
    });

    let send_another = use_callback(move |()| {
        send_address.set(String::new());
        send_amount.set(String::new());
        fee_choice.set(FeeChoice::Normal);
        custom_fee_rate.set(String::new());
        send_error.set(None);
        send_plan.set(None);
        send_signed.set(None);
        broadcast_state.set(BroadcastState::Idle);
        sending.set(false);
    });

    let confirm_send = use_callback(move |()| {
        if sending() {
            return;
        }
        let Some(plan) = send_plan.read().clone() else {
            return;
        };
        let signed = {
            let wallet = session.read();
            match wallet.as_ref() {
                Some(wallet) => sign_send_plan(wallet, &plan).map_err(|error| error.to_string()),
                None => Err("The wallet session is not available. Start over.".to_owned()),
            }
        };

        let signed = match signed {
            Ok(signed) => signed,
            Err(message) => {
                send_error.set(Some(message));
                return;
            }
        };

        send_signed.set(Some(signed.clone()));
        send_error.set(None);
        broadcast_state.set(BroadcastState::Broadcasting);
        sending.set(true);

        let raw_transaction_hex = signed.raw_hex.clone();
        let expected_txid = signed.txid.clone();
        let fee_sat = signed.fee_sat;
        let vsize = signed.vsize;
        let change_sat = signed.change_sat;
        spawn(async move {
            match api::broadcast_transaction(raw_transaction_hex).await {
                Ok(response) if response.txid == expected_txid => {
                    if change_sat > 0 {
                        if let Some(wallet) = session.write().as_mut() {
                            if let Err(error) = wallet.record_change_used() {
                                wallet_error.set(Some(error.to_string()));
                            }
                        }
                    }

                    let addresses = session
                        .read()
                        .as_ref()
                        .map(WalletSession::scan_addresses)
                        .unwrap_or_default();
                    broadcast_state.set(BroadcastState::Success {
                        txid: response.txid,
                        fee_sat,
                        vsize,
                    });
                    sending.set(false);
                    request_wallet_refresh(
                        addresses,
                        true,
                        snapshot,
                        summary,
                        loading,
                        refresh_error,
                        refresh_id,
                    );
                }
                Ok(_) => {
                    sending.set(false);
                    broadcast_state.set(BroadcastState::Failed(
                        "The provider returned a different transaction ID. Refresh before retrying."
                            .to_owned(),
                    ));
                }
                Err(error) => {
                    sending.set(false);
                    broadcast_state.set(BroadcastState::Failed(error.message().to_owned()));
                }
            }
        });
    });

    let content = match stage() {
        WalletStage::Landing => rsx! {
            header { class: "page-header",
                div { class: "page-header__meta",
                    p { class: "eyebrow", "My Wallet" }
                    span { class: "pill pill--orange", "Bitcoin testnet" }
                }
                h1 { "A testnet wallet you control" }
                p {
                    "Create or import a wallet, track its balance, receive with a QR code, and send testnet bitcoin signed in this browser."
                }
            }

            if let Some(message) = wallet_error.read().as_ref() {
                div { class: "wallet-alert wallet-alert--error", role: "alert",
                    WarningIcon {}
                    div {
                        strong { "Wallet not created" }
                        p { "{message}" }
                    }
                }
            }

            div { class: "wallet-banner",
                div { class: "wallet-banner__icon", WalletIcon {} }
                div {
                    h2 { "A full testnet wallet" }
                    p {
                        "Balances, multiple receiving addresses, fee-aware sends, and transaction history. Testnet coins have no real value."
                    }
                }
            }

            div { class: "wallet-actions",
                section { class: "wallet-action-card",
                    div { class: "wallet-action-card__icon", WalletIcon {} }
                    h2 { "Create a new wallet" }
                    p {
                        "Generate 12 recovery words in your browser, then use the wallet to receive and send testnet bitcoin."
                    }
                    PrimaryButton {
                        onclick: move |_| start_create.call(()),
                        "Create a wallet"
                    }
                }

                section { class: "wallet-action-card",
                    div { class: "wallet-action-card__icon", ImportIcon {} }
                    h2 { "Import a wallet" }
                    p {
                        "Enter a 12- or 24-word English BIP39 recovery phrase you already use for testnet."
                    }
                    SecondaryButton {
                        onclick: move |_| start_import.call(()),
                        "Import a wallet"
                    }
                }
            }

            aside { class: "wallet-safety",
                ShieldIcon {}
                div {
                    strong { "Keys stay in this tab only." }
                    p {
                        "BitSabi never saves wallet words or keys. Refreshing or closing the tab clears the wallet, so back up your recovery phrase."
                    }
                }
            }
        },

        WalletStage::Create => rsx! {
            header { class: "page-header",
                div { class: "page-header__meta",
                    p { class: "eyebrow", "Create wallet" }
                    span { class: "pill pill--orange", "Bitcoin testnet" }
                }
                h1 { "Write down your recovery phrase" }
                p {
                    "These 12 words can restore this wallet. Write them down in order and keep them private."
                }
            }

            div { class: "wallet-alert wallet-alert--warning",
                WarningIcon {}
                div {
                    strong { "Anyone with these words can control the wallet." }
                    p {
                        "BitSabi cannot recover the phrase for you. Do not share it, screenshot it, or send it to anyone."
                    }
                }
            }

            if let Some(wallet) = created_wallet.read().as_ref() {
                section { class: "wallet-recovery", aria_label: "Recovery phrase",
                    ol { class: "recovery-words",
                        for (index, word) in wallet.phrase().split_whitespace().enumerate() {
                            li { key: "{index}",
                                span { class: "recovery-words__number", "{index + 1}" }
                                span { class: "recovery-words__word", "{word}" }
                            }
                        }
                    }
                    p { class: "wallet-recovery__note",
                        "This phrase is shown only during this flow. After you continue, the signing key stays in this tab's memory so you can send. Refreshing clears it."
                    }
                }

                if let Some(message) = wallet_error.read().as_ref() {
                    p { class: "wallet-error", role: "alert", "{message}" }
                }

                label { class: "wallet-confirm",
                    input {
                        r#type: "checkbox",
                        checked: backup_confirmed(),
                        onchange: move |event| backup_confirmed.set(event.checked()),
                    }
                    span {
                        strong { "I have written down these 12 words in order." }
                        small { "I understand BitSabi cannot recover them for me." }
                    }
                }

                div { class: "wallet-flow__actions",
                    SecondaryButton {
                        onclick: move |_| reset_to_landing.call(()),
                        ArrowLeftIcon {}
                        "Cancel"
                    }
                    PrimaryButton {
                        disabled: !backup_confirmed(),
                        onclick: move |_| confirm_backup.call(()),
                        "Continue to wallet"
                    }
                }
            } else {
                div { class: "wallet-alert wallet-alert--error", role: "alert",
                    WarningIcon {}
                    div {
                        strong { "Wallet state was cleared." }
                        p { "Start again to generate a new recovery phrase." }
                    }
                }
                div { class: "wallet-flow__actions",
                    SecondaryButton {
                        onclick: move |_| reset_to_landing.call(()),
                        ArrowLeftIcon {}
                        "Back to wallet"
                    }
                }
            }
        },

        WalletStage::Import => rsx! {
            header { class: "page-header",
                div { class: "page-header__meta",
                    p { class: "eyebrow", "Import wallet" }
                    span { class: "pill pill--orange", "Bitcoin testnet" }
                }
                h1 { "Restore a testnet wallet" }
                p {
                    "Enter your 12- or 24-word English BIP39 recovery phrase. It is checked locally and never sent to the server."
                }
            }

            div { class: "wallet-alert wallet-alert--warning",
                WarningIcon {}
                div {
                    strong { "Never enter a phrase for real bitcoin here." }
                    p {
                        "This browser wallet is for testnet practice only. Use a phrase that does not control mainnet funds."
                    }
                }
            }

            form {
                class: "wallet-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    import_wallet.call(());
                },
                div { class: "wallet-field",
                    label { r#for: "wallet-import-phrase", "Recovery phrase" }
                    textarea {
                        id: "wallet-import-phrase",
                        class: if wallet_error.read().is_some() {
                            "wallet-textarea wallet-textarea--error"
                        } else {
                            "wallet-textarea"
                        },
                        rows: "4",
                        value: "{import_phrase.read().as_str()}",
                        placeholder: "Enter 12 or 24 words separated by spaces",
                        autocomplete: "off",
                        autocapitalize: "none",
                        spellcheck: false,
                        aria_invalid: if wallet_error.read().is_some() { "true" } else { "false" },
                        aria_describedby: "wallet-import-help wallet-import-error",
                        oninput: move |event| {
                            import_phrase.set(Zeroizing::new(event.value()));
                            wallet_error.set(None);
                        },
                    }
                    p { class: "wallet-field__help",
                        "English words only. Optional BIP39 passphrases are not supported in this milestone."
                    }
                    div { class: "wallet-field__meta", id: "wallet-import-help",
                        span { "Words are separated by spaces." }
                        span {
                            if import_word_count == 1 {
                                "1 word entered"
                            } else {
                                "{import_word_count} words entered"
                            }
                        }
                    }
                }

                if let Some(message) = wallet_error.read().as_ref() {
                    p { class: "wallet-error", id: "wallet-import-error", role: "alert",
                        "{message}"
                    }
                }

                div { class: "wallet-flow__actions",
                    SecondaryButton {
                        onclick: move |_| reset_to_landing.call(()),
                        ArrowLeftIcon {}
                        "Cancel"
                    }
                    button {
                        class: "button button--primary",
                        r#type: "submit",
                        disabled: import_word_count == 0,
                        "Import wallet"
                    }
                }
            }
        },

        WalletStage::Ready => {
            let receive_addresses = session
                .read()
                .as_ref()
                .map(|wallet| {
                    wallet
                        .visible_receive_addresses()
                        .iter()
                        .map(|address| (address.index(), address.address().to_owned()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let current_receive_address = receive_addresses
                .last()
                .map(|(_, address)| address.clone())
                .unwrap_or_default();
            let wallet_summary = summary.read().clone();
            let snapshot_fees = snapshot.read().as_ref().map(|snapshot| snapshot.fees);
            let active_panel = panel();
            let broadcast = broadcast_state();
            let plan = send_plan.read().clone();
            let current_copy_state = copy_state();
            let current_copied_address = copied_address.read().clone();
            let history_count = wallet_summary
                .as_ref()
                .map_or(0, |summary| summary.transactions.len());
            let visible_history = visible_tx_count().min(history_count);

            rsx! {
                header { class: "page-header",
                    div { class: "page-header__meta",
                        p { class: "eyebrow", "Testnet wallet" }
                        span { class: "pill pill--orange", "Bitcoin testnet" }
                    }
                    h1 { "Your testnet wallet" }
                    p {
                        "Keys stay in this tab's memory so sends can be signed locally. Balance and history come from the public testnet chain."
                    }
                }

                div { class: "wallet-toolbar",
                    div { class: "wallet-toolbar__meta",
                        span { class: "wallet-toolbar__dot" }
                        span {
                            if loading() {
                                "Refreshing testnet data..."
                            } else if let Some(summary) = wallet_summary.as_ref() {
                                "Chain tip #{summary.tip_height} · {summary.provider}"
                            } else {
                                "Waiting for testnet data"
                            }
                        }
                    }
                    div { class: "wallet-toolbar__actions",
                        SecondaryButton {
                            disabled: loading(),
                            onclick: move |_| manual_refresh.call(()),
                            RefreshIcon {}
                            if loading() { "Refreshing" } else { "Refresh" }
                        }
                        SecondaryButton {
                            onclick: move |_| reset_to_landing.call(()),
                            "Start over"
                        }
                    }
                }

                nav { class: "wallet-tabs", aria_label: "Wallet sections",
                    button {
                        class: tab_class(active_panel == WalletPanel::Overview),
                        r#type: "button",
                        onclick: move |_| panel.set(WalletPanel::Overview),
                        "Overview"
                    }
                    button {
                        class: tab_class(active_panel == WalletPanel::Receive),
                        r#type: "button",
                        onclick: move |_| panel.set(WalletPanel::Receive),
                        "Receive"
                    }
                    button {
                        class: tab_class(active_panel == WalletPanel::Send),
                        r#type: "button",
                        onclick: move |_| panel.set(WalletPanel::Send),
                        "Send"
                    }
                    button {
                        class: tab_class(active_panel == WalletPanel::History),
                        r#type: "button",
                        onclick: move |_| panel.set(WalletPanel::History),
                        "History"
                    }
                }

                if let Some(message) = wallet_error.read().as_ref() {
                    div { class: "wallet-alert wallet-alert--error", role: "alert",
                        WarningIcon {}
                        div {
                            strong { "Wallet error" }
                            p { "{message}" }
                        }
                    }
                }

                if let Some(message) = refresh_error.read().as_ref() {
                    div { class: "wallet-alert wallet-alert--warning", role: "status",
                        WarningIcon {}
                        div {
                            strong {
                                if wallet_summary.is_some() {
                                    "Showing the last wallet snapshot"
                                } else {
                                    "Wallet data is unavailable"
                                }
                            }
                            p { "{message}" }
                            button {
                                class: "text-button",
                                r#type: "button",
                                onclick: move |_| manual_refresh.call(()),
                                "Try again"
                            }
                        }
                    }
                }

                if active_panel == WalletPanel::Overview {
                    if let Some(summary) = wallet_summary.as_ref() {
                        section { class: "wallet-balance-grid", aria_label: "Wallet balance",
                            article { class: "wallet-balance-card wallet-balance-card--primary",
                                span { class: "wallet-balance-card__label", "Confirmed balance" }
                                strong { class: "wallet-balance-card__value", {format_signed_sats(summary.confirmed_sat)} }
                                p { "Available to spend now" }
                            }
                            article { class: "wallet-balance-card",
                                span { class: "wallet-balance-card__label", "Pending" }
                                strong {
                                    class: if summary.pending_sat < 0 {
                                        "wallet-balance-card__value wallet-balance-card__value--negative"
                                    } else {
                                        "wallet-balance-card__value"
                                    },
                                    {format_signed_sats(summary.pending_sat)}
                                }
                                p {
                                    if summary.pending_sat == 0 {
                                        "No unconfirmed changes"
                                    } else {
                                        "Waiting for confirmation"
                                    }
                                }
                            }
                            article { class: "wallet-balance-card",
                                span { class: "wallet-balance-card__label", "Total" }
                                strong { class: "wallet-balance-card__value", {format_signed_sats(summary.total_sat)} }
                                p { "{summary.utxos.len()} outputs · {summary.transactions.len()} transactions" }
                            }
                        }
                    } else if loading() {
                        div { class: "wallet-loading", role: "status",
                            RefreshIcon {}
                            div {
                                strong { "Loading your testnet wallet" }
                                p { "Checking your receiving and change addresses for balances and history." }
                            }
                        }
                    } else {
                        div { class: "wallet-empty",
                            WalletIcon {}
                            h2 { "Wallet data has not loaded yet" }
                            p { "Refresh to check the testnet chain." }
                            SecondaryButton {
                                onclick: move |_| manual_refresh.call(()),
                                RefreshIcon {}
                                "Refresh"
                            }
                        }
                    }

                    div { class: "wallet-actions",
                        section { class: "wallet-action-card",
                            div { class: "wallet-action-card__icon", SendReceiveIcon {} }
                            h2 { "Send testnet bitcoin" }
                            p { "Choose a recipient and fee rate. The transaction is signed locally before broadcast." }
                            PrimaryButton {
                                onclick: move |_| panel.set(WalletPanel::Send),
                                "Send"
                            }
                        }
                        section { class: "wallet-action-card",
                            div { class: "wallet-action-card__icon", WalletIcon {} }
                            h2 { "Receive testnet bitcoin" }
                            p { "Share your current address or generate another receiving address." }
                            SecondaryButton {
                                onclick: move |_| panel.set(WalletPanel::Receive),
                                "Receive"
                            }
                        }
                    }

                    if !current_receive_address.is_empty() {
                        section { class: "wallet-address-card", aria_labelledby: "overview-address-heading",
                            div { class: "wallet-address-card__header",
                                div {
                                    span { class: "wallet-address-card__label", "Current receiving address" }
                                    h2 { id: "overview-address-heading", "Share this address to get testnet coins" }
                                }
                                span { class: "pill pill--orange", "BIP84 · Native SegWit" }
                            }
                            code { class: "wallet-address", "{current_receive_address}" }
                            div { class: "wallet-address-card__actions",
                                {
                                    let address = current_receive_address.clone();
                                    rsx! {
                                        SecondaryButton {
                                            onclick: move |_| copy_text_to_clipboard(
                                                address.clone(),
                                                copy_state,
                                                copied_address,
                                            ),
                                            CopyIcon {}
                                            if current_copy_state == CopyState::Copied
                                                && current_copied_address.as_deref() == Some(current_receive_address.as_str())
                                            {
                                                "Copied"
                                            } else {
                                                "Copy address"
                                            }
                                        }
                                    }
                                }
                                SecondaryButton {
                                    onclick: move |_| panel.set(WalletPanel::Receive),
                                    "Receive"
                                }
                            }
                            if current_copied_address.as_deref() == Some(current_receive_address.as_str()) {
                                if current_copy_state == CopyState::Copied {
                                    p { class: "copy-feedback copy-feedback--success", role: "status",
                                        "Address copied to the clipboard."
                                    }
                                } else if current_copy_state == CopyState::Failed {
                                    p { class: "copy-feedback copy-feedback--error", role: "status",
                                        "Could not copy automatically. Select the address and copy it manually."
                                    }
                                }
                            }
                        }
                    }

                    if let Some(summary) = wallet_summary.as_ref() {
                        section { class: "wallet-panel",
                            div { class: "wallet-panel__header",
                                div {
                                    h2 { "Recent activity" }
                                    p { if summary.transactions.is_empty() { "No transactions yet." } else { "Latest testnet transactions touching this wallet." } }
                                }
                                if !summary.transactions.is_empty() {
                                    SecondaryButton {
                                        onclick: move |_| panel.set(WalletPanel::History),
                                        "View all"
                                    }
                                }
                            }
                            if summary.transactions.is_empty() {
                                div { class: "wallet-empty wallet-empty--compact",
                                    SendReceiveIcon {}
                                    h3 { "No transactions yet" }
                                    p { "Receive testnet bitcoin to see it appear here after the provider indexes it." }
                                }
                            } else {
                                div { class: "wallet-history",
                                    for transaction in summary.transactions.iter().take(OVERVIEW_TX_LIMIT) {
                                        TransactionRow { key: "{transaction.txid}", transaction: transaction.clone() }
                                    }
                                }
                            }
                        }
                    }
                }

                if active_panel == WalletPanel::Receive {
                    section { class: "wallet-panel",
                        div { class: "wallet-panel__header",
                            div {
                                h2 { "Receive testnet bitcoin" }
                                p { "Share the QR code or copy the current address. Testnet coins have no real value." }
                            }
                            span { class: "pill pill--orange", "BIP84 · Native SegWit" }
                        }

                        if current_receive_address.is_empty() {
                            div { class: "wallet-empty",
                                WarningIcon {}
                                h3 { "No receiving address is available" }
                                p { "Start over to create or import the wallet again." }
                            }
                        } else {
                            div { class: "wallet-receive-grid",
                                div { class: "wallet-qr-wrap",
                                    AddressQrCode { value: current_receive_address.clone() }
                                    span { "Scan to receive" }
                                }
                                div { class: "wallet-receive-details",
                                    span { class: "wallet-address-card__label", "Current receiving address" }
                                    code { class: "wallet-address", "{current_receive_address}" }
                                    div { class: "wallet-address-card__actions",
                                        {
                                            let address = current_receive_address.clone();
                                            rsx! {
                                                PrimaryButton {
                                                    onclick: move |_| copy_text_to_clipboard(
                                                        address.clone(),
                                                        copy_state,
                                                        copied_address,
                                                    ),
                                                    CopyIcon {}
                                                    if current_copy_state == CopyState::Copied
                                                        && current_copied_address.as_deref() == Some(current_receive_address.as_str())
                                                    {
                                                        "Copied"
                                                    } else {
                                                        "Copy address"
                                                    }
                                                }
                                            }
                                        }
                                        SecondaryButton {
                                            onclick: move |_| generate_address.call(()),
                                            "Generate new address"
                                        }
                                    }
                                    if current_copied_address.as_deref() == Some(current_receive_address.as_str()) {
                                        if current_copy_state == CopyState::Copied {
                                            p { class: "copy-feedback copy-feedback--success", role: "status",
                                                "Address copied to the clipboard."
                                            }
                                        } else if current_copy_state == CopyState::Failed {
                                            p { class: "copy-feedback copy-feedback--error", role: "status",
                                                "Could not copy automatically. Select the address and copy it manually."
                                            }
                                        }
                                    }
                                    p { class: "wallet-field__help",
                                        "Only send testnet bitcoin to this address. Mainnet funds sent here may be lost."
                                    }
                                }
                            }
                        }

                        aside { class: "wallet-safety",
                            ShieldIcon {}
                            div {
                                strong { "Balances scan a 20-address gap." }
                                p {
                                    "Previously generated receiving addresses remain in this wallet and stay part of balance and history checks. Refresh after receiving funds."
                                }
                            }
                        }
                    }

                    section { class: "wallet-panel",
                        div { class: "wallet-panel__header",
                            div {
                                h2 { "Receiving addresses" }
                                p { "Newest first. Every address here remains accessible for this session." }
                            }
                        }
                        if receive_addresses.is_empty() {
                            p { class: "wallet-field__help", "No receiving addresses are available." }
                        } else {
                            ol { class: "wallet-address-list",
                                for (position, (index, address)) in receive_addresses.iter().rev().enumerate() {
                                    li { key: "{index}",
                                        div { class: "wallet-address-list__main",
                                            span { class: "wallet-address-list__index", "#{position + 1}" }
                                            code { "{address}" }
                                        }
                                        div { class: "wallet-address-list__actions",
                                            {
                                                let address = address.clone();
                                                let copied = current_copied_address.as_deref() == Some(address.as_str());
                                                rsx! {
                                                    button {
                                                        class: "text-button",
                                                        r#type: "button",
                                                        onclick: move |_| copy_text_to_clipboard(
                                                            address.clone(),
                                                            copy_state,
                                                            copied_address,
                                                        ),
                                                        if copied && current_copy_state == CopyState::Copied {
                                                            "Copied"
                                                        } else {
                                                            "Copy"
                                                        }
                                                    }
                                                }
                                            }
                                            if position == 0 {
                                                span { class: "pill pill--orange", "Current" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if active_panel == WalletPanel::Send {
                    section { class: "wallet-panel",
                        div { class: "wallet-panel__header",
                            div {
                                h2 { "Send testnet bitcoin" }
                                p { "The transaction is signed locally in this tab. BitSabi only broadcasts the signed transaction." }
                            }
                            if let Some(summary) = wallet_summary.as_ref() {
                                span { class: "wallet-panel__balance", "Spendable {format_sats(summary.confirmed_sat.max(0) as u64)}" }
                            }
                        }

                        if snapshot_fees.is_none() {
                            div { class: "wallet-empty",
                                RefreshIcon {}
                                h3 { "Wallet data is still loading" }
                                p { "Balance and fee estimates are needed before a transaction can be prepared." }
                                SecondaryButton {
                                    onclick: move |_| manual_refresh.call(()),
                                    RefreshIcon {}
                                    "Refresh"
                                }
                            }
                        } else if let BroadcastState::Success { txid, fee_sat, vsize } = broadcast {
                            div { class: "wallet-success", role: "status",
                                div { class: "wallet-success__icon", CheckIcon {} }
                                div {
                                    h2 { "Transaction broadcast" }
                                    p {
                                        "The testnet provider accepted the signed transaction. It now needs to be included in a block."
                                    }
                                    code { class: "wallet-success__txid", "{txid}" }
                                    div { class: "wallet-review__rows",
                                        div { class: "wallet-review__row", span { "Network fee" } strong { "{format_sats(fee_sat)}" } }
                                        div { class: "wallet-review__row", span { "Transaction size" } strong { "{vsize} vB" } }
                                    }
                                    a {
                                        class: "text-link",
                                        href: "https://mempool.space/testnet/tx/{txid}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        "View on mempool.space"
                                        ExternalLinkIcon {}
                                    }
                                    p { class: "wallet-field__help",
                                        "Refresh to update confirmations. Broadcast success does not guarantee confirmation."
                                    }
                                    div { class: "wallet-flow__actions",
                                        SecondaryButton {
                                            onclick: move |_| manual_refresh.call(()),
                                            RefreshIcon {}
                                            "Refresh wallet"
                                        }
                                        PrimaryButton {
                                            onclick: move |_| send_another.call(()),
                                            "Send another"
                                        }
                                    }
                                }
                            }
                        } else if let BroadcastState::Failed(message) = broadcast {
                            div { class: "wallet-alert wallet-alert--error", role: "alert",
                                WarningIcon {}
                                div {
                                    strong { "The transaction was not broadcast" }
                                    p { "{message}" }
                                    p { class: "wallet-field__help",
                                        "Your keys and signed transaction are still in this tab. You can retry or edit the transaction."
                                    }
                                    div { class: "wallet-flow__actions",
                                        SecondaryButton {
                                            onclick: move |_| edit_send.call(()),
                                            "Edit transaction"
                                        }
                                        PrimaryButton {
                                            disabled: sending(),
                                            onclick: move |_| confirm_send.call(()),
                                            if sending() { "Broadcasting..." } else { "Retry broadcast" }
                                        }
                                    }
                                }
                            }
                        } else if broadcast == BroadcastState::Broadcasting {
                            div { class: "wallet-loading", role: "status",
                                RefreshIcon {}
                                div {
                                    strong { "Broadcasting your transaction" }
                                    p { "Waiting for the testnet provider to accept it. Do not close this tab." }
                                }
                            }
                        } else if let Some(plan) = plan {
                            div { class: "wallet-review",
                                div { class: "wallet-panel__header",
                                    div {
                                        h2 { "Review transaction" }
                                        p { "Bitcoin transactions cannot be reversed. Check the recipient and fee." }
                                    }
                                    span { class: "pill pill--orange", "Signed locally" }
                                }
                                div { class: "wallet-review__rows",
                                    div { class: "wallet-review__row", span { "Recipient" } code { "{plan.recipient}" } }
                                    div { class: "wallet-review__row", span { "Amount" } strong { "{format_sats(plan.amount_sat)}" } }
                                    div { class: "wallet-review__row", span { "Fee rate" } strong { "{plan.fee_rate_sat_vb} sat/vB" } }
                                    div { class: "wallet-review__row", span { "Network fee" } strong { "{format_sats(plan.fee_sat)}" } }
                                    div { class: "wallet-review__row", span { "Inputs" } strong { "{plan.input_count}" } }
                                    if plan.change_sat > 0 {
                                        div { class: "wallet-review__row", span { "Change" } strong { "{format_sats(plan.change_sat)}" } }
                                    }
                                    div { class: "wallet-review__row wallet-review__row--total", span { "Total spent" } strong { "{format_sats(plan.amount_sat + plan.fee_sat)}" } }
                                }
                                if let Some(message) = send_error.read().as_ref() {
                                    p { class: "wallet-error", role: "alert", "{message}" }
                                }
                                div { class: "wallet-flow__actions",
                                    SecondaryButton {
                                        onclick: move |_| edit_send.call(()),
                                        ArrowLeftIcon {}
                                        "Edit"
                                    }
                                    PrimaryButton {
                                        disabled: sending(),
                                        onclick: move |_| confirm_send.call(()),
                                        if sending() { "Signing and broadcasting..." } else { "Confirm and send" }
                                    }
                                }
                            }
                        } else {
                            form {
                                class: "wallet-form",
                                onsubmit: move |event| {
                                    event.prevent_default();
                                    review_send.call(());
                                },
                                div { class: "wallet-field",
                                    label { r#for: "wallet-send-address", "Recipient testnet address" }
                                    input {
                                        id: "wallet-send-address",
                                        class: "wallet-input",
                                        r#type: "text",
                                        value: "{send_address}",
                                        placeholder: "tb1q...",
                                        autocomplete: "off",
                                        autocapitalize: "none",
                                        spellcheck: false,
                                        oninput: move |event| {
                                            send_address.set(event.value());
                                            send_plan.set(None);
                                            send_signed.set(None);
                                            send_error.set(None);
                                            broadcast_state.set(BroadcastState::Idle);
                                        },
                                    }
                                    p { class: "wallet-field__help",
                                        "Mainnet addresses are rejected. Double-check the network before sending."
                                    }
                                }

                                div { class: "wallet-form-grid",
                                    div { class: "wallet-field",
                                        label { r#for: "wallet-send-amount", "Amount (BTC)" }
                                        input {
                                            id: "wallet-send-amount",
                                            class: "wallet-input",
                                            r#type: "text",
                                            inputmode: "decimal",
                                            value: "{send_amount}",
                                            placeholder: "0.00050000",
                                            autocomplete: "off",
                                            oninput: move |event| {
                                                send_amount.set(event.value());
                                                send_plan.set(None);
                                                send_signed.set(None);
                                                send_error.set(None);
                                                broadcast_state.set(BroadcastState::Idle);
                                            },
                                        }
                                        p { class: "wallet-field__help",
                                            "Up to 8 decimal places. A network fee is added on top."
                                        }
                                    }
                                    div { class: "wallet-field",
                                        label { "Fee rate" }
                                        div { class: "wallet-fee-options", role: "group", aria_label: "Fee rate",
                                            for (choice, label, rate) in fee_options(snapshot_fees.unwrap()) {
                                                button {
                                                    key: "{label}",
                                                    class: if fee_choice() == choice {
                                                        "wallet-fee-option wallet-fee-option--active"
                                                    } else {
                                                        "wallet-fee-option"
                                                    },
                                                    r#type: "button",
                                                    onclick: move |_| {
                                                        fee_choice.set(choice);
                                                        send_plan.set(None);
                                                        send_signed.set(None);
                                                        send_error.set(None);
                                                        broadcast_state.set(BroadcastState::Idle);
                                                    },
                                                    strong { "{label}" }
                                                    span { "{rate} sat/vB" }
                                                }
                                            }
                                            button {
                                                class: if fee_choice() == FeeChoice::Custom {
                                                    "wallet-fee-option wallet-fee-option--active"
                                                } else {
                                                    "wallet-fee-option"
                                                },
                                                r#type: "button",
                                                onclick: move |_| {
                                                    fee_choice.set(FeeChoice::Custom);
                                                    send_plan.set(None);
                                                    send_signed.set(None);
                                                    send_error.set(None);
                                                    broadcast_state.set(BroadcastState::Idle);
                                                },
                                                strong { "Custom" }
                                                span { "sat/vB" }
                                            }
                                        }
                                        if fee_choice() == FeeChoice::Custom {
                                            input {
                                                class: "wallet-input wallet-input--compact",
                                                r#type: "text",
                                                inputmode: "numeric",
                                                value: "{custom_fee_rate}",
                                                placeholder: "1–1000",
                                                oninput: move |event| {
                                                    custom_fee_rate.set(event.value());
                                                    send_plan.set(None);
                                                    send_signed.set(None);
                                                    send_error.set(None);
                                                    broadcast_state.set(BroadcastState::Idle);
                                                },
                                            }
                                        }
                                    }
                                }

                                if let Some(summary) = wallet_summary.as_ref() {
                                    if summary.spendable_utxos().is_empty() {
                                        p { class: "wallet-field__help",
                                            "No confirmed testnet outputs are available to spend. Receive funds and wait for a confirmation."
                                        }
                                    } else if summary.pending_sat != 0 {
                                        p { class: "wallet-field__help",
                                            "Pending balance changes are not spendable until they confirm."
                                        }
                                    }
                                }

                                if let Some(message) = send_error.read().as_ref() {
                                    p { class: "wallet-error", role: "alert", "{message}" }
                                }

                                div { class: "wallet-flow__actions",
                                    SecondaryButton {
                                        onclick: move |_| panel.set(WalletPanel::Overview),
                                        ArrowLeftIcon {}
                                        "Back"
                                    }
                                    button {
                                        class: "button button--primary",
                                        r#type: "submit",
                                        disabled: sending(),
                                        "Review transaction"
                                    }
                                }
                            }
                        }
                    }
                }

                if active_panel == WalletPanel::History {
                    section { class: "wallet-panel",
                        div { class: "wallet-panel__header",
                            div {
                                h2 { "Transaction history" }
                                p { "Recent confirmed and pending transactions touching this wallet." }
                            }
                            SecondaryButton {
                                disabled: loading(),
                                onclick: move |_| manual_refresh.call(()),
                                RefreshIcon {}
                                if loading() { "Refreshing" } else { "Refresh" }
                            }
                        }

                        if let Some(summary) = wallet_summary.as_ref() {
                            if summary.history_truncated {
                                div { class: "wallet-alert wallet-alert--warning",
                                    WarningIcon {}
                                    div {
                                        strong { "Showing the provider's recent window" }
                                        p {
                                            "Some addresses have more transactions than the chain API returns at once. Newer activity is shown first."
                                        }
                                    }
                                }
                            }

                            if summary.transactions.is_empty() {
                                div { class: "wallet-empty",
                                    SendReceiveIcon {}
                                    h3 { "No transactions yet" }
                                    p { "Receive testnet bitcoin or send a transaction to populate history." }
                                }
                            } else {
                                div { class: "wallet-history",
                                    for transaction in summary.transactions.iter().take(visible_history) {
                                        TransactionRow { key: "{transaction.txid}", transaction: transaction.clone() }
                                    }
                                }
                                div { class: "wallet-pagination",
                                    span {
                                        "Showing {visible_history} of {summary.transactions.len()}"
                                        if summary.history_truncated { " loaded transactions" } else { "" }
                                    }
                                    div { class: "wallet-pagination__actions",
                                        if visible_history < summary.transactions.len() {
                                            SecondaryButton {
                                                onclick: move |_| visible_tx_count.set(visible_history + HISTORY_PAGE_SIZE),
                                                "Show older"
                                            }
                                        }
                                        if visible_history > HISTORY_PAGE_SIZE {
                                            SecondaryButton {
                                                onclick: move |_| visible_tx_count.set(HISTORY_PAGE_SIZE),
                                                "Show less"
                                            }
                                        }
                                    }
                                }
                            }
                        } else if loading() {
                            div { class: "wallet-loading", role: "status",
                                RefreshIcon {}
                                div {
                                    strong { "Loading transaction history" }
                                    p { "Checking all derived wallet addresses." }
                                }
                            }
                        } else {
                            div { class: "wallet-empty",
                                ClockIcon {}
                                h3 { "History is unavailable" }
                                p { "Refresh to try loading transactions again." }
                                SecondaryButton {
                                    onclick: move |_| manual_refresh.call(()),
                                    RefreshIcon {}
                                    "Refresh"
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    rsx! {
        div { class: "page wallet-page", {content} }
    }
}

#[component]
fn TransactionRow(transaction: HistoryTransaction) -> Element {
    let direction = transaction.direction();
    let direction_class = match direction {
        TransactionDirection::Received => "wallet-tx--received",
        TransactionDirection::Sent => "wallet-tx--sent",
        TransactionDirection::SelfTransfer => "wallet-tx--self",
    };
    let direction_label = match direction {
        TransactionDirection::Received => "Received",
        TransactionDirection::Sent => "Sent",
        TransactionDirection::SelfTransfer => "Self-transfer",
    };
    let amount = match direction {
        TransactionDirection::Received => {
            format!("+{}", format_sats(transaction.net_value.unsigned_abs()))
        }
        TransactionDirection::Sent => {
            format!("-{}", format_sats(transaction.net_value.unsigned_abs()))
        }
        TransactionDirection::SelfTransfer => "0 BTC".to_owned(),
    };
    let status = if transaction.status.confirmed {
        match transaction.confirmations {
            1 => "1 confirmation".to_owned(),
            confirmations => format!("{confirmations} confirmations"),
        }
    } else {
        "Pending".to_owned()
    };
    let short_txid = short_txid(&transaction.txid);
    let explorer_url = format!(
        "https://mempool.space/testnet/tx/{}",
        transaction.txid
    );

    rsx! {
        article { class: "wallet-tx {direction_class}",
            div { class: "wallet-tx__icon",
                if transaction.status.confirmed {
                    SendReceiveIcon {}
                } else {
                    ClockIcon {}
                }
            }
            div { class: "wallet-tx__body",
                div { class: "wallet-tx__head",
                    div {
                        strong { "{direction_label}" }
                        span { class: "wallet-tx__status", "{status}" }
                    }
                    strong { class: "wallet-tx__amount", "{amount}" }
                }
                div { class: "wallet-tx__meta",
                    if transaction.status.confirmed {
                        if let Some(height) = transaction.status.block_height {
                            span { "Block #{height}" }
                        }
                    } else {
                        span { "Waiting in the mempool" }
                    }
                    if matches!(
                        direction,
                        TransactionDirection::Sent | TransactionDirection::SelfTransfer
                    ) {
                        if let Some(fee) = transaction.fee {
                            span { "Fee {format_sats(fee)}" }
                        }
                    }
                    a {
                        class: "wallet-tx__link",
                        href: "{explorer_url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        title: "View transaction on mempool.space",
                        "{short_txid}"
                        ExternalLinkIcon {}
                    }
                }
            }
        }
    }
}

fn tab_class(active: bool) -> &'static str {
    if active {
        "wallet-tab wallet-tab--active"
    } else {
        "wallet-tab"
    }
}

fn fee_options(fees: FeeEstimates) -> [(FeeChoice, &'static str, u64); 3] {
    [
        (FeeChoice::Fastest, "Fastest", fees.fastest_sat_vb),
        (FeeChoice::Normal, "Normal", fees.half_hour_sat_vb),
        (FeeChoice::Economy, "Economy", fees.economy_sat_vb),
    ]
}

fn fee_rate_for_choice(
    choice: FeeChoice,
    custom: &str,
    fees: FeeEstimates,
) -> Result<u64, String> {
    let rate = match choice {
        FeeChoice::Fastest => fees.fastest_sat_vb,
        FeeChoice::Normal => fees.half_hour_sat_vb,
        FeeChoice::Economy => fees.economy_sat_vb,
        FeeChoice::Custom => custom
            .trim()
            .parse::<u64>()
            .map_err(|_| "Enter a custom fee rate in sat/vB.".to_owned())?,
    };

    if !(1..=1_000).contains(&rate) {
        return Err("The fee rate must be between 1 and 1000 sat/vB.".to_owned());
    }

    Ok(rate)
}

fn short_txid(txid: &str) -> String {
    if txid.len() <= 18 {
        return txid.to_owned();
    }
    format!("{}…{}", &txid[..8], &txid[txid.len() - 6..])
}

fn request_wallet_refresh(
    addresses: Vec<String>,
    force: bool,
    mut snapshot: Signal<Option<WalletSnapshot>>,
    mut summary: Signal<Option<WalletSummary>>,
    mut loading: Signal<bool>,
    mut refresh_error: Signal<Option<String>>,
    mut refresh_id: Signal<u64>,
) {
    if addresses.is_empty() || (loading() && !force) {
        return;
    }

    let request_id = refresh_id().wrapping_add(1);
    refresh_id.set(request_id);
    loading.set(true);
    refresh_error.set(None);

    spawn(async move {
        let result = api::fetch_wallet_snapshot(addresses).await;
        if refresh_id() != request_id {
            return;
        }

        match result {
            Ok(wallet_snapshot) => {
                let wallet_summary = WalletSummary::from_snapshot(&wallet_snapshot);
                summary.set(Some(wallet_summary));
                snapshot.set(Some(wallet_snapshot));
                refresh_error.set(None);
            }
            Err(error) => refresh_error.set(Some(error.message().to_owned())),
        }
        loading.set(false);
    });
}

fn copy_text_to_clipboard(
    address: String,
    mut copy_state: Signal<CopyState>,
    mut copied_address: Signal<Option<String>>,
) {
    let Ok(encoded_address) = serde_json::to_string(&address) else {
        copy_state.set(CopyState::Failed);
        copied_address.set(Some(address));
        return;
    };

    let script = format!(
        r#"
        try {{
            if (!navigator.clipboard || typeof navigator.clipboard.writeText !== "function") {{
                return false;
            }}
            await navigator.clipboard.writeText({encoded_address});
            return true;
        }} catch (_error) {{
            return false;
        }}
        "#
    );

    copied_address.set(Some(address.clone()));
    copy_state.set(CopyState::Idle);

    spawn(async move {
        let copied = document::eval(&script)
            .await
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        if copied_address().as_deref() != Some(address.as_str()) {
            return;
        }

        copy_state.set(if copied {
            CopyState::Copied
        } else {
            CopyState::Failed
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_ids_are_shortened_for_display() {
        assert_eq!(
            short_txid("1111111111111111111111111111111111111111111111111111111111111111"),
            "11111111…111111"
        );
        assert_eq!(short_txid("abc"), "abc");
    }

    #[test]
    fn fee_choices_map_to_estimates() {
        let fees = FeeEstimates {
            fastest_sat_vb: 5,
            half_hour_sat_vb: 3,
            hour_sat_vb: 2,
            economy_sat_vb: 1,
            minimum_sat_vb: 1,
        };

        assert_eq!(
            fee_rate_for_choice(FeeChoice::Fastest, "", fees).unwrap(),
            5
        );
        assert_eq!(
            fee_rate_for_choice(FeeChoice::Normal, "", fees).unwrap(),
            3
        );
        assert_eq!(
            fee_rate_for_choice(FeeChoice::Economy, "", fees).unwrap(),
            1
        );
        assert_eq!(
            fee_rate_for_choice(FeeChoice::Custom, "7", fees).unwrap(),
            7
        );
        assert!(fee_rate_for_choice(FeeChoice::Custom, "", fees).is_err());
        assert!(fee_rate_for_choice(FeeChoice::Custom, "0", fees).is_err());
    }
}
