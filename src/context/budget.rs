use crate::memory::OUTPUT_MARGIN;

/// What a request may spend, in tokens.
///
/// [`allowed`](Self::allowed) is what the history may take: the
/// [`window`](Self::window) less [`reserved`](Self::reserved) and
/// [`preamble`](Self::preamble). The defaults are a 4,096-token answer
/// margin, an 8,000-token tail, no preamble allowance, and 1,000 tokens for
/// one tool result.
///
/// ```
/// use rig::context::Budget;
///
/// let budget = Budget::new(8_000).reserved(2_000);
/// assert_eq!(budget.allowed(), 6_000);
/// assert!(budget.over(6_001));
/// assert!(!budget.fits(6_000));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    window: u64,
    reserved: u64,
    tail: u64,
    preamble: u64,
    per_result: u64,
}

impl Budget {
    /// A budget for a model whose context window is `window` tokens.
    pub fn new(window: u64) -> Self {
        Self {
            window,
            reserved: OUTPUT_MARGIN,
            tail: 8_000,
            preamble: 0,
            per_result: 1_000,
        }
    }

    /// Room to keep free for the answer and its thinking.
    #[must_use = "the setting applies to the returned value"]
    pub fn reserved(mut self, tokens: u64) -> Self {
        self.reserved = tokens;
        self
    }

    /// Tokens at the end of the history that trimming never touches. A tail
    /// larger than the history keeps all of it.
    #[must_use = "the setting applies to the returned value"]
    pub fn tail(mut self, tokens: u64) -> Self {
        self.tail = tokens;
        self
    }

    /// Tokens the system prompt and tool definitions take, which the history
    /// budget does not include.
    #[must_use = "the setting applies to the returned value"]
    pub fn preamble(mut self, tokens: u64) -> Self {
        self.preamble = tokens;
        self
    }

    /// The most tokens one tool result may keep before it is stubbed.
    #[must_use = "the setting applies to the returned value"]
    pub fn per_result(mut self, tokens: u64) -> Self {
        self.per_result = tokens;
        self
    }

    /// The model's context window.
    pub fn window(&self) -> u64 {
        self.window
    }

    /// Tokens kept free for the answer.
    pub fn reserved_tokens(&self) -> u64 {
        self.reserved
    }

    /// Tokens at the end of the history that trimming never touches.
    pub fn tail_tokens(&self) -> u64 {
        self.tail
    }

    /// Tokens the preamble is assumed to take.
    pub fn preamble_tokens(&self) -> u64 {
        self.preamble
    }

    /// The most tokens one tool result may keep.
    pub fn per_result_tokens(&self) -> u64 {
        self.per_result
    }

    /// What the history may take.
    pub fn allowed(&self) -> u64 {
        self.window
            .saturating_sub(self.reserved)
            .saturating_sub(self.preamble)
    }

    /// Whether `tokens` of history leave room for the answer.
    pub fn fits(&self, tokens: u64) -> bool {
        tokens <= self.allowed()
    }

    /// Whether `tokens` of history leave no room for the answer.
    pub fn over(&self, tokens: u64) -> bool {
        !self.fits(tokens)
    }
}
