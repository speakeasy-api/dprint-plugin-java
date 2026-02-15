import java.util.List;
import java.util.ArrayList;
import java.util.stream.Collectors;

public class ComplexExample {
    private List<String> items;
    private int count;

    public ComplexExample() {
        this.items = new ArrayList<>();
        this.count = 0;
    }

    public void addItem(String item) {
        if (item != null && !item.isEmpty()) {
            items.add(item);
            count++;
        }
    }

    public List<String> getFilteredItems(String prefix) {
        return items.stream().filter(item -> item.startsWith(prefix)).collect(Collectors.toList());
    }

    public void processItems() {
        for (int i = 0; i < items.size(); i++) {
            String item = items.get(i);
            if (item.length() > 5) {
                System.out.println("Long item: " + item);
            } else {
                System.out.println("Short item: " + item);
            }
        }
    }

    public static void main(String[] args) {
        ComplexExample example = new ComplexExample();
        example.addItem("apple");
        example.addItem("banana");
        example.addItem("cherry");
        for (String item : example.getFilteredItems("a")) {
            System.out.println(item);
        }
        example.processItems();
    }
}
