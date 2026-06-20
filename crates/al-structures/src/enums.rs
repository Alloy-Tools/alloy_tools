//! Simple helper enums used by other utility types.

/// Represents control flow decisions for helper utilities.
///
/// # Examples
///
/// ```
/// use al_structures::enums::ControlFlow;
/// 
/// let result = ControlFlow::Continue; 
///
/// match result {
///     ControlFlow::Continue => println!("Keep going"),
///     ControlFlow::Break => println!("Done"),
/// }
/// ```
#[derive(Debug)]
pub enum ControlFlow {
    /// Continue normal execution.
    Continue,
    /// Break out.
    Break,
}

/// A either/or enum used to indicate one of two values.
///
/// # Examples
///
/// ```
/// use al_structures::enums::Which;
///
/// let result: Which<u8, &str> = Which::A(42);
///
/// match result {
///     Which::A(num) => println!("Got a number: {}", num),
///     Which::B(text) => println!("Got text: {}", text),
/// }
/// ```
#[derive(Debug)]
pub enum Which<A, B> {
    /// The first variant.
    A(A),
    /// The second variant.
    B(B),
}
