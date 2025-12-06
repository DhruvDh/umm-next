"""Diff grading test: script that causes a runtime error."""


def main():
    result = 1 / 0  # ZeroDivisionError
    print(result)


if __name__ == "__main__":
    main()
