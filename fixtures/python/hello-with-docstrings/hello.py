"""Script with docstrings for docs grading."""


def greet(name: str) -> str:
    """Greet a person by name.

    Args:
        name: The name of the person.

    Returns:
        A greeting string.
    """
    return f"Hello, {name}!"


def add(a: int, b: int) -> int:
    """Add two numbers.

    Args:
        a: First number.
        b: Second number.

    Returns:
        The sum of a and b.
    """
    return a + b


class Person:
    """Represents a person.

    Attributes:
        name: The person's name.
        age: The person's age.
    """

    def __init__(self, name: str, age: int):
        """Initialize a Person.

        Args:
            name: The person's name.
            age: The person's age.
        """
        self.name = name
        self.age = age

    def birthday(self) -> None:
        """Increment the person's age by one."""
        self.age += 1

    def greet(self) -> str:
        """Return a greeting from this person.

        Returns:
            A greeting string.
        """
        return f"Hi, I'm {self.name}!"


def main():
    """Main entry point."""
    print(greet("World"))
    print(add(2, 3))
    person = Person("Alice", 30)
    print(person.greet())


if __name__ == "__main__":
    main()
