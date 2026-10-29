//! Lesson copy, quiz answers, and lightweight lesson metadata.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LessonIcon {
    Bitcoin,
    Network,
    Wallet,
    Shield,
    SendReceive,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LessonSection {
    pub heading: &'static str,
    pub body: &'static [&'static str],
    pub example: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quiz {
    pub question: &'static str,
    pub options: [&'static str; 3],
    pub answer: usize,
    pub explanation: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lesson {
    pub slug: &'static str,
    pub number: usize,
    pub title: &'static str,
    pub summary: &'static str,
    pub minutes: u8,
    pub icon: LessonIcon,
    pub sections: &'static [LessonSection],
    pub quiz: Quiz,
}

pub const LESSONS: [Lesson; 5] = [
    Lesson {
        slug: "what-is-bitcoin",
        number: 1,
        title: "What is Bitcoin?",
        summary: "A plain-English introduction to digital money that no company owns.",
        minutes: 3,
        icon: LessonIcon::Bitcoin,
        sections: &[
            LessonSection {
                heading: "Money without a central owner",
                body: &[
                    "Bitcoin is a digital form of money. People can send it directly to one another without a bank or payment company sitting in the middle.",
                    "No single company or government controls the Bitcoin network. Instead, thousands of computers around the world follow the same rules and keep copies of the same public record.",
                ],
                example: Some(
                    "Think of it like a public ledger that everyone can read, but no one can quietly rewrite.",
                ),
            },
            LessonSection {
                heading: "There is a fixed supply",
                body: &[
                    "New bitcoin is issued according to rules written into the software. The supply is capped at 21 million bitcoin, so no one can simply print more.",
                    "One bitcoin can be divided into very small amounts, so you do not need to buy a whole coin to use it.",
                ],
                example: None,
            },
            LessonSection {
                heading: "Bitcoin is not a company",
                body: &[
                    "There is no Bitcoin office, customer support line, or chief executive. The network is open-source software run by its participants.",
                    "That also means no one can reverse a transaction for you. Learning how to check addresses and keep your keys safe is part of using Bitcoin responsibly.",
                ],
                example: None,
            },
        ],
        quiz: Quiz {
            question: "Who controls the Bitcoin network?",
            options: [
                "A single company",
                "The participants running the shared rules",
                "The Nigerian central bank",
            ],
            answer: 1,
            explanation: "Bitcoin has no central owner. Its rules are enforced by the network of participants, not by one company or government.",
        },
    },
    Lesson {
        slug: "how-bitcoin-works",
        number: 2,
        title: "How Bitcoin works",
        summary: "Transactions, blocks, and why the public record is shared.",
        minutes: 3,
        icon: LessonIcon::Network,
        sections: &[
            LessonSection {
                heading: "Transactions move value",
                body: &[
                    "A Bitcoin transaction says that some bitcoin should move from one address to another. The sender authorises it with a private key, and the network checks that the sender really controls that bitcoin.",
                    "Once a transaction is valid, it is broadcast to other computers on the network. They can all inspect it using public information.",
                ],
                example: None,
            },
            LessonSection {
                heading: "Blocks make a chain",
                body: &[
                    "Valid transactions are grouped into blocks. Each new block is linked to the previous one, forming a chain that is very difficult to rewrite.",
                    "Miners do the work of adding blocks. They compete to solve a computational puzzle, and the winner gets to propose the next block.",
                ],
                example: Some(
                    "A confirmation is a block added after your transaction. More confirmations usually mean more certainty, but they do not make a transaction completely instant.",
                ),
            },
            LessonSection {
                heading: "The network reaches agreement",
                body: &[
                    "Nodes independently check the rules, including that no one spends the same bitcoin twice. If a node sees a block that breaks the rules, it rejects it.",
                    "This shared checking is why Bitcoin does not need a central referee. The public record is the source of truth.",
                ],
                example: None,
            },
        ],
        quiz: Quiz {
            question: "What is a Bitcoin block?",
            options: [
                "A group of valid transactions linked to earlier blocks",
                "A password that unlocks a wallet",
                "A bank statement from a Bitcoin company",
            ],
            answer: 0,
            explanation: "A block is a batch of valid transactions. Blocks are linked together, which is why Bitcoin uses the term blockchain.",
        },
    },
    Lesson {
        slug: "what-is-a-wallet",
        number: 3,
        title: "What is a Bitcoin wallet?",
        summary: "Addresses, private keys, and what a wallet actually stores.",
        minutes: 3,
        icon: LessonIcon::Wallet,
        sections: &[
            LessonSection {
                heading: "A wallet manages keys",
                body: &[
                    "A Bitcoin wallet does not hold bitcoin inside it. The bitcoin is recorded on the public blockchain. Your wallet manages the keys that prove you can spend it.",
                    "A public address is safe to share when you want to receive bitcoin. A private key must stay secret because it gives control over the funds.",
                ],
                example: Some(
                    "An address is like a mailbox number. A private key is like the key that opens the mailbox.",
                ),
            },
            LessonSection {
                heading: "One seed, many keys",
                body: &[
                    "Most modern wallets use a recovery phrase, usually 12 or 24 words. That phrase can restore the keys for many addresses.",
                    "Anyone with your recovery phrase can control your bitcoin. Never type it into a website, send it in a chat, or store it in plain text on your phone.",
                ],
                example: None,
            },
            LessonSection {
                heading: "Self-custody means responsibility",
                body: &[
                    "With a self-custody wallet, you control the keys. That gives you freedom, but it also means there is no company that can reset your password or recover funds for you.",
                    "BitSabi will keep wallet keys inside your browser in a later testnet build. You will still need to back up your recovery phrase safely and understand the risks.",
                ],
                example: None,
            },
        ],
        quiz: Quiz {
            question: "Which piece of information must stay secret?",
            options: [
                "Your receiving address",
                "Your private key or recovery phrase",
                "The current Bitcoin price",
            ],
            answer: 1,
            explanation: "Private keys and recovery phrases control your bitcoin. A receiving address is public and can be shared safely.",
        },
    },
    Lesson {
        slug: "keep-bitcoin-safe",
        number: 4,
        title: "How to keep Bitcoin safe",
        summary: "Recovery phrases, common scams, and the meaning of self-custody.",
        minutes: 3,
        icon: LessonIcon::Shield,
        sections: &[
            LessonSection {
                heading: "Protect the recovery phrase",
                body: &[
                    "Write your recovery phrase on paper or another offline backup. Keep it somewhere private, and consider more than one secure location.",
                    "Never photograph it, email it, or type it into a website that asks for it. A genuine wallet will not need your phrase to receive bitcoin.",
                ],
                example: None,
            },
            LessonSection {
                heading: "Slow down before sending",
                body: &[
                    "Before sending bitcoin, check the address carefully and start with a small test amount when possible. A wrong address can mean the money is gone.",
                    "Be suspicious of anyone who creates urgency, promises guaranteed profits, or asks you to install software so they can help. These are common scam patterns.",
                ],
                example: Some(
                    "A request to \"validate\" your wallet by entering your recovery phrase is almost always an attempt to steal it.",
                ),
            },
            LessonSection {
                heading: "Self-custody is a trade-off",
                body: &[
                    "Keeping your own keys removes reliance on an exchange or bank, but it also removes their ability to help you recover from mistakes.",
                    "Use small amounts while learning. On BitSabi, wallet practice will use testnet bitcoin so a mistake cannot cost real money.",
                ],
                example: None,
            },
        ],
        quiz: Quiz {
            question: "A stranger offers to help you recover a wallet if you share your 12 words. What should you do?",
            options: [
                "Share the words so they can help quickly",
                "Refuse and keep the phrase private",
                "Post the words in a public group",
            ],
            answer: 1,
            explanation: "Never share a recovery phrase. Anyone with those words can take control of the wallet. No legitimate support person needs them.",
        },
    },
    Lesson {
        slug: "send-and-receive",
        number: 5,
        title: "Sending and receiving Bitcoin",
        summary: "Addresses, fees, confirmations, and common mistakes.",
        minutes: 3,
        icon: LessonIcon::SendReceive,
        sections: &[
            LessonSection {
                heading: "Receiving bitcoin",
                body: &[
                    "To receive bitcoin, share your receiving address or a QR code created from it. The sender uses that address to create a transaction.",
                    "You can reuse an address, but many wallets create a fresh one for each payment so it is easier to keep your activity private.",
                ],
                example: None,
            },
            LessonSection {
                heading: "Sending bitcoin",
                body: &[
                    "To send bitcoin, enter the recipient's address, choose an amount, and review the fee. The fee pays the network for processing the transaction.",
                    "A higher fee can help a transaction confirm sooner, but it is not a guarantee. Always check the full amount before approving.",
                ],
                example: None,
            },
            LessonSection {
                heading: "Confirmations and mistakes",
                body: &[
                    "After broadcast, a transaction waits in the mempool until a miner includes it in a block. Each later block adds another confirmation.",
                    "Bitcoin transactions cannot be reversed by a support team. If you send to the wrong address, there is usually no way to get the money back.",
                ],
                example: Some(
                    "Send a small test amount first when you are using a new address or a new wallet.",
                ),
            },
        ],
        quiz: Quiz {
            question: "What should you do before sending bitcoin to a new address?",
            options: [
                "Send the full amount immediately",
                "Check the address carefully, then consider a small test amount",
                "Share your recovery phrase with the recipient",
            ],
            answer: 1,
            explanation: "Bitcoin payments cannot normally be reversed. Check the address and use a small test amount when you can.",
        },
    },
];

pub fn find_lesson(slug: &str) -> Option<&'static Lesson> {
    LESSONS.iter().find(|lesson| lesson.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn lesson_slugs_are_unique() {
        let slugs: HashSet<_> = LESSONS.iter().map(|lesson| lesson.slug).collect();
        assert_eq!(slugs.len(), LESSONS.len());
    }

    #[test]
    fn quiz_answers_are_in_range() {
        for lesson in LESSONS {
            assert!(lesson.quiz.answer < lesson.quiz.options.len());
            assert!(!lesson.sections.is_empty());
        }
    }
}
