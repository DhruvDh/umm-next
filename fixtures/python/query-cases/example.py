"""Query grading test: example with various Python constructs."""


class Calculator:
    """A simple calculator class."""

    def __init__(self, initial_value: float = 0):
        """Initialize the calculator.

        Args:
            initial_value: Starting value.
        """
        self.value = initial_value

    def add(self, n: float) -> float:
        """Add n to the current value.

        Args:
            n: Number to add.

        Returns:
            The new value.
        """
        self.value += n
        return self.value

    def multiply(self, n: float) -> float:
        """Multiply the current value by n.

        Args:
            n: Multiplier.

        Returns:
            The new value.
        """
        self.value *= n
        return self.value


def sum_with_loop(numbers: list[int]) -> int:
    """Sum numbers using a for loop.

    Args:
        numbers: List of numbers to sum.

    Returns:
        The sum.
    """
    total = 0
    for n in numbers:
        total += n
    return total


def sum_with_while(numbers: list[int]) -> int:
    """Sum numbers using a while loop.

    Args:
        numbers: List of numbers to sum.

    Returns:
        The sum.
    """
    total = 0
    i = 0
    while i < len(numbers):
        total += numbers[i]
        i += 1
    return total


def process_value(value: int) -> str:
    """Process a value with conditional logic.

    Args:
        value: Value to process.

    Returns:
        A string representation.
    """
    if value > 0:
        return "positive"
    elif value < 0:
        return "negative"
    else:
        return "zero"


def squares_comprehension(n: int) -> list[int]:
    """Return squares using list comprehension.

    Args:
        n: Upper limit (exclusive).

    Returns:
        List of squares.
    """
    return [x * x for x in range(n)]


def main():
    """Main entry point."""
    calc = Calculator(10)
    print(f"Initial: {calc.value}")
    print(f"After add(5): {calc.add(5)}")
    print(f"After multiply(2): {calc.multiply(2)}")

    nums = [1, 2, 3, 4, 5]
    print(f"Sum (for): {sum_with_loop(nums)}")
    print(f"Sum (while): {sum_with_while(nums)}")
    print(f"Process 5: {process_value(5)}")
    print(f"Squares: {squares_comprehension(5)}")


if __name__ == "__main__":
    main()
