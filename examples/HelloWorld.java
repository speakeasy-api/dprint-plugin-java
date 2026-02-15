public class HelloWorld {
    private String message;

    public HelloWorld(String message) {
        this.message = message;
    }

    public void printMessage() {
        if (message != null) {
            System.out.println(message);
        } else {
            System.out.println("No message");
        }
    }

    public static void main(String[] args) {
        HelloWorld hello = new HelloWorld("Hello, World!");
        hello.printMessage();
    }
}
