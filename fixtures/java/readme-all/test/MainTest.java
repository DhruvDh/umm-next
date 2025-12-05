import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

/** Basic JUnit5 coverage for the Rune sample project. */
public class MainTest {
    @Test
    void greets() {
        assertEquals("Hello from Rune", Main.greet());
    }

    @Test
    void sums() {
        assertEquals(6, new Main().sumTo(4));
    }
}
