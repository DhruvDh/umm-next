/** Simple greetings and math helpers used by Rune sample graders. */
public class Main {
    /**
     * Constructs a new {@code Main} instance.
     */
    public Main() {
        // default
    }

    /**
     * Returns the greeting expected by the diff grader and tests.
     *
     * @return greeting string
     */
    public static String greet() {
        return "Hello from Rune";
    }

    /**
     * Entry point prints the canonical greeting.
     *
     * @param args standard CLI arguments (unused)
     */
    public static void main(String[] args) {
        System.out.println(greet());
    }

    /**
     * Deterministic loop for query grader to match and for tests to assert.
     *
     * @param n upper bound (exclusive)
     * @return arithmetic series sum from 0 to n - 1
     */
    public int sumTo(int n) {
        int sum = 0;
        for (int i = 0; i < n; i++) {
            sum += i;
        }
        return sum;
    }
}
