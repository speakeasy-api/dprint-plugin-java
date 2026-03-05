public class TestCase {
    void method() {
        JCRNodeWrapper mountPointNode = jcrMountPointNode.getVirtualMountPointNode();
        final JCRStoreProvider provider = externalProviderFactory.mountProvider(mountPointNode);
        if (!provider.isAvailable(true)) {
            System.out.println();
        }
    }
}
