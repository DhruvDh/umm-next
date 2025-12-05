package query;

public class Example {
  public void foo(int n) {
    int sum = 0;
    for (int i = 0; i < n; i++) {
      sum += i;
    }
    if (sum > 10) {
      System.out.println("big");
    } else {
      System.out.println("small");
    }
    while (sum > 0) {
      sum--;
    }
  }

  public static void main(String[] args) {
    Example ex = new Example();
    ex.foo(3);
  }
}
