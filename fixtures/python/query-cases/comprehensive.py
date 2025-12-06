"""Comprehensive Python example for testing all query grader patterns."""

import os
from pathlib import Path


# Decorator example
def my_decorator(func):
    """A simple decorator."""
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)
    return wrapper


@my_decorator
def decorated_function():
    """A decorated function."""
    return "decorated"


# Lambda example
square = lambda x: x * x


# List comprehension
squares = [x * x for x in range(10)]

# Dict comprehension
square_dict = {x: x * x for x in range(5)}

# Set comprehension
unique_squares = {x * x for x in range(-5, 6)}

# Generator expression
gen = (x * x for x in range(10))


def generator_function():
    """A generator function using yield."""
    for i in range(10):
        yield i * 2


def uses_with_statement():
    """Function that uses a context manager."""
    with open("/dev/null", "r") as f:
        pass


def uses_try_except():
    """Function with exception handling."""
    try:
        result = 1 / 0
    except ZeroDivisionError:
        result = 0
    return result


def uses_assert():
    """Function with assert statement."""
    x = 5
    assert x > 0, "x must be positive"
    return x


def uses_raise():
    """Function that raises an exception."""
    if True:
        raise ValueError("example error")


def for_loop_function():
    """Function using for loop."""
    total = 0
    for i in range(10):
        total += i
    return total


def while_loop_function():
    """Function using while loop."""
    count = 0
    while count < 10:
        count += 1
    return count


def if_statement_function(x):
    """Function using if statement."""
    if x > 0:
        return "positive"
    elif x < 0:
        return "negative"
    else:
        return "zero"


def non_recursive_sum(numbers):
    """Sum without recursion - uses for loop."""
    total = 0
    for n in numbers:
        total += n
    return total


def recursive_sum(numbers):
    """Sum WITH recursion."""
    if not numbers:
        return 0
    return numbers[0] + recursive_sum(numbers[1:])


class Calculator:
    """A simple calculator class."""

    def __init__(self, value=0):
        self.value = value

    def add(self, n):
        self.value += n
        return self.value


def main():
    """Main entry point."""
    print("Comprehensive example")
    print(f"Squares: {squares}")
    print(f"Lambda result: {square(5)}")
    print(f"Recursive sum: {recursive_sum([1, 2, 3])}")


if __name__ == "__main__":
    main()
