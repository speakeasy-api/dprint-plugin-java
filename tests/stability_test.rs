//! Stability (idempotency) tests for formatting decisions that depend on source positions.
//!
//! Each test exercises a specific category of source-position-dependent formatting logic.
//! Tests verify 3-pass stability: format(format(format(input))) == format(input).
//! Failing tests identify instability patterns to fix — they serve as a roadmap.

use std::path::Path;

use dprint_core::configuration::NewLineKind;
use dprint_plugin_java::configuration::Configuration;
use dprint_plugin_java::format_text::format_text;

fn default_config() -> Configuration {
    Configuration {
        line_width: 120,
        indent_width: 4,
        use_tabs: false,
        new_line_kind: NewLineKind::LineFeed,
        format_javadoc: false,
        method_chain_threshold: 80,
        inline_lambdas: true,
    }
}

/// Assert that formatting is idempotent (3-pass stable).
/// Formats the input, then formats the result two more times, asserting no further changes.
fn assert_idempotent(name: &str, input: &str) {
    assert_idempotent_with_config(name, input, &default_config());
}

fn assert_idempotent_with_config(name: &str, input: &str, config: &Configuration) {
    let pass1 = format_text(Path::new("Test.java"), input, config)
        .unwrap()
        .unwrap_or_else(|| input.to_string());

    let pass2 = format_text(Path::new("Test.java"), &pass1, config)
        .unwrap()
        .unwrap_or_else(|| pass1.clone());

    if pass1 != pass2 {
        panic!(
            "Stability test '{}' FAILED: pass 1 != pass 2\n\n--- pass 1 ---\n{}\n--- pass 2 ---\n{}\n--- end ---",
            name, pass1, pass2
        );
    }

    let pass3 = format_text(Path::new("Test.java"), &pass2, config)
        .unwrap()
        .unwrap_or_else(|| pass2.clone());

    if pass2 != pass3 {
        panic!(
            "Stability test '{}' FAILED: pass 2 != pass 3\n\n--- pass 2 ---\n{}\n--- pass 3 ---\n{}\n--- end ---",
            name, pass2, pass3
        );
    }
}

// =============================================================================
// Category A: String concatenation wrapping (exercises binary expr width calc)
// =============================================================================

#[test]
fn stability_string_concat_in_method_arg() {
    assert_idempotent(
        "string_concat_in_method_arg",
        r#"class Test {
    void test() {
        plugin.setConfiguration(Xpp3DomBuilder.build(new StringReader("<config><Depends>" + depends + "</Depends></config>")));
    }
}
"#,
    );
}

#[test]
fn stability_string_concat_in_variable_init() {
    assert_idempotent(
        "string_concat_in_variable_init",
        r#"class Test {
    void test() {
        String url = Jahia.getContextPath() + "/cms/" + servlet + "/" + workspace + "/" + locale;
    }
}
"#,
    );
}

#[test]
fn stability_string_concat_deeply_nested() {
    assert_idempotent(
        "string_concat_deeply_nested",
        r#"class Test {
    void test() {
        new HTML("<b>" + Messages.get("label.name") + ":</b> " + SafeHtmlUtils.htmlEscape(name));
    }
}
"#,
    );
}

#[test]
fn stability_string_concat_multiline_arg() {
    assert_idempotent(
        "string_concat_multiline_arg",
        r#"class Test {
    void test() {
        logger.warn("Failed to process request for path=" + path + " with session=" + session.getId() + " and user=" + user.getName());
    }
}
"#,
    );
}

#[test]
fn stability_string_concat_in_conditional() {
    assert_idempotent(
        "string_concat_in_conditional",
        r#"class Test {
    void test() {
        if (!"Expected value: " + expected + " but got: " + actual + " for field: " + fieldName.equals(result)) {
            throw new AssertionError("mismatch");
        }
    }
}
"#,
    );
}

// =============================================================================
// Category B: throws clause wrapping (exercises method signature width calc)
// =============================================================================

#[test]
fn stability_throws_clause_after_params() {
    assert_idempotent(
        "throws_clause_after_params",
        r#"class Test {
    public String getUrl(String servlet, String workspace, String locale,
            boolean findDisplayable) throws RepositoryException {
        return "";
    }
}
"#,
    );
}

#[test]
fn stability_throws_clause_many_exceptions() {
    assert_idempotent(
        "throws_clause_many_exceptions",
        r#"class Test {
    public static void main(String[] args) throws IOException, SQLException, ClassNotFoundException, InterruptedException {
        System.out.println("hello");
    }
}
"#,
    );
}

#[test]
fn stability_throws_clause_long_method_name() {
    assert_idempotent(
        "throws_clause_long_method_name",
        r#"class Test {
    public synchronized Map<String, Object> processAndTransformConfiguration(ConfigurationRequest request, ValidationContext context) throws ConfigurationException, ValidationException {
        return Collections.emptyMap();
    }
}
"#,
    );
}

#[test]
fn stability_constructor_throws() {
    assert_idempotent(
        "constructor_throws",
        r#"class VeryLongClassName {
    public VeryLongClassName(String parameterOne, String parameterTwo, String parameterThree) throws IOException, RepositoryException {
        this.parameterOne = parameterOne;
    }
}
"#,
    );
}

// =============================================================================
// Category C: new expression in assignments (exercises chain prefix width)
// =============================================================================

#[test]
fn stability_new_expr_in_assignment() {
    assert_idempotent(
        "new_expr_in_assignment",
        r#"class Test {
    void test() {
        SimpleCredentials credentials = new SimpleCredentials(userID, getSystemPass(userID, deniedPaths).toCharArray());
    }
}
"#,
    );
}

#[test]
fn stability_new_expr_long_type() {
    assert_idempotent(
        "new_expr_long_type",
        r#"class Test {
    void test() {
        Object[] result = getTargetNodeType(nodeTypeName, (GWTJahiaNodeType) child, displayedNodeTypes);
    }
}
"#,
    );
}

#[test]
fn stability_new_expr_with_chained_method() {
    assert_idempotent(
        "new_expr_with_chained_method",
        r#"class Test {
    void test() {
        String result = new StringBuilder().append("prefix").append(value).append("suffix").toString();
    }
}
"#,
    );
}

#[test]
fn stability_new_expr_generic_type_assignment() {
    assert_idempotent(
        "new_expr_generic_type_assignment",
        r#"class Test {
    void test() {
        Map<String, List<GWTJahiaNode>> nodesByType = new HashMap<String, List<GWTJahiaNode>>();
    }
}
"#,
    );
}

// =============================================================================
// Category D: Generic type wrapping (exercises type args prefix width)
// =============================================================================

#[test]
fn stability_generic_type_in_foreach() {
    assert_idempotent(
        "generic_type_in_foreach",
        r#"class Test {
    void test() {
        for (Map.Entry<String, List<DefaultEventListener>> entry : listeners.entrySet()) {
            process(entry);
        }
    }
}
"#,
    );
}

#[test]
fn stability_nested_generic_type_variable() {
    assert_idempotent(
        "nested_generic_type_variable",
        r#"class Test {
    void test() {
        CompletableFuture<AsyncRequestOperation<BinaryAndStringUploadRequest, BinaryAndStringUploadResponse>> op = createOp();
    }
}
"#,
    );
}

#[test]
fn stability_generic_return_type_long_method() {
    assert_idempotent(
        "generic_return_type_long_method",
        r#"class Test {
    public Map<String, List<Map<String, Object>>> processComplexDataStructure(List<Map<String, Object>> inputData) {
        return Collections.emptyMap();
    }
}
"#,
    );
}

// =============================================================================
// Category E: Method chain at various prefix widths
// =============================================================================

#[test]
fn stability_chain_with_long_lhs_type() {
    assert_idempotent(
        "chain_with_long_lhs_type",
        r#"class Test {
    void test() {
        final JCRStoreProvider provider = externalProviderFactory.mountProvider(mountPointNode);
    }
}
"#,
    );
}

#[test]
fn stability_chain_deeply_nested_field_access() {
    assert_idempotent(
        "chain_deeply_nested_field_access",
        r#"class Test {
    void test() {
        Element row = pageTree.getView().getRow(selectionModel.getRightClickSelectionModel().getSelectedItem());
    }
}
"#,
    );
}

#[test]
fn stability_chain_multiple_methods_near_limit() {
    assert_idempotent(
        "chain_multiple_methods_near_limit",
        r#"class Test {
    void test() {
        String result = factory.createBuilder().withParam(key, value).withTimeout(Duration.ofSeconds(30)).build();
    }
}
"#,
    );
}

#[test]
fn stability_chain_stream_operations() {
    assert_idempotent(
        "chain_stream_operations",
        r#"class Test {
    void test() {
        List<String> names = employees.stream().filter(e -> e.isActive()).map(Employee::getName).sorted().collect(Collectors.toList());
    }
}
"#,
    );
}

#[test]
fn stability_chain_builder_pattern_long() {
    assert_idempotent(
        "chain_builder_pattern_long",
        r#"class Test {
    void test() {
        HttpRequest request = HttpRequest.newBuilder().uri(URI.create(baseUrl + "/api/v1/endpoint")).header("Authorization", "Bearer " + token).header("Content-Type", "application/json").timeout(Duration.ofSeconds(30)).GET().build();
    }
}
"#,
    );
}

#[test]
fn stability_chain_with_lambda_arg() {
    assert_idempotent(
        "chain_with_lambda_arg",
        r#"class Test {
    void test() {
        items.stream().filter(item -> item.getStatus() == Status.ACTIVE && item.getCreatedDate().isAfter(cutoff)).map(item -> item.getName()).collect(Collectors.toList());
    }
}
"#,
    );
}

// =============================================================================
// Category F: Annotation argument wrapping
// =============================================================================

#[test]
fn stability_annotation_suppress_warnings() {
    assert_idempotent(
        "annotation_suppress_warnings",
        r#"class Test {
    @SuppressWarnings({"unchecked", "rawtypes", "deprecation", "serial", "finally"})
    void test() {}
}
"#,
    );
}

#[test]
fn stability_annotation_request_mapping() {
    assert_idempotent(
        "annotation_request_mapping",
        r#"class Test {
    @RequestMapping(value = "/api/v1/very-long-endpoint-name", method = RequestMethod.GET, produces = MediaType.APPLICATION_JSON_VALUE)
    void test() {}
}
"#,
    );
}

#[test]
fn stability_annotation_multiline_values() {
    assert_idempotent(
        "annotation_multiline_values",
        r#"class Test {
    @JsonPropertyOrder({"id", "name", "email", "phone", "address", "city", "state", "zipCode", "country"})
    static class UserDto {}
}
"#,
    );
}

// =============================================================================
// Category G: Ternary expression wrapping
// =============================================================================

#[test]
fn stability_ternary_in_return() {
    assert_idempotent(
        "ternary_in_return",
        r#"class Test {
    boolean test() {
        return httpClientConfiguration.getRedactedHeaders() != null && !httpClientConfiguration.getRedactedHeaders().isEmpty();
    }
}
"#,
    );
}

#[test]
fn stability_ternary_in_assignment_long_lhs() {
    assert_idempotent(
        "ternary_in_assignment_long_lhs",
        r#"class Test {
    void test() {
        String msg = httpClient.getConfig() != null ? httpClient.getConfig().getStatusMessage() : "Unknown error message";
    }
}
"#,
    );
}

#[test]
fn stability_ternary_nested() {
    assert_idempotent(
        "ternary_nested",
        r#"class Test {
    void test() {
        String result = condition1 ? (condition2 ? "value_a" : "value_b") : (condition3 ? "value_c" : "value_d");
    }
}
"#,
    );
}

#[test]
fn stability_ternary_with_method_calls() {
    assert_idempotent(
        "ternary_with_method_calls",
        r#"class Test {
    void test() {
        Object value = Optional.ofNullable(input).isPresent() ? transformer.transform(input.getValue()) : defaultProvider.getDefault();
    }
}
"#,
    );
}

// =============================================================================
// Category H: Formal parameter wrapping
// =============================================================================

#[test]
fn stability_params_many_near_limit() {
    assert_idempotent(
        "params_many_near_limit",
        r#"class Test {
    public void processRequest(HttpServletRequest request, HttpServletResponse response, ServletContext context, String path) {
        // body
    }
}
"#,
    );
}

#[test]
fn stability_params_with_annotations() {
    assert_idempotent(
        "params_with_annotations",
        r#"class Test {
    public MyService(@Inject ConfigProvider config, @Named("primary") DataSource dataSource, Logger logger) {
        this.config = config;
    }
}
"#,
    );
}

#[test]
fn stability_params_generic_types() {
    assert_idempotent(
        "params_generic_types",
        r#"class Test {
    public <T extends Comparable<T>> List<T> mergeAndSort(List<T> firstList, List<T> secondList, Comparator<T> comparator) {
        return Collections.emptyList();
    }
}
"#,
    );
}

#[test]
fn stability_params_long_with_defaults() {
    assert_idempotent(
        "params_long_with_defaults",
        r#"class Test {
    public ResponseEntity<ApiResponse<UserDto>> updateUserProfile(@PathVariable("userId") Long userId, @RequestBody @Valid UpdateProfileRequest request, @AuthenticationPrincipal UserDetails currentUser) {
        return ResponseEntity.ok(null);
    }
}
"#,
    );
}

// =============================================================================
// Category I: Argument list wrapping with mixed content
// =============================================================================

#[test]
fn stability_args_with_lambda() {
    assert_idempotent(
        "args_with_lambda",
        r#"class Test {
    void test() {
        CompletableFuture<Response> future = client.sendAsync(request, response -> {
            return response.body();
        });
    }
}
"#,
    );
}

#[test]
fn stability_args_nested_method_calls() {
    assert_idempotent(
        "args_nested_method_calls",
        r#"class Test {
    void test() {
        session.importXML(targetPath, is, ImportUUIDBehavior.IMPORT_UUID_CREATE_NEW);
    }
}
"#,
    );
}

#[test]
fn stability_args_many_string_literals() {
    assert_idempotent(
        "args_many_string_literals",
        r#"class Test {
    void test() {
        logger.info("Processing request: method={}, path={}, user={}, session={}", request.getMethod(), request.getPath(), user.getName(), session.getId());
    }
}
"#,
    );
}

#[test]
fn stability_args_mixed_types_near_limit() {
    assert_idempotent(
        "args_mixed_types_near_limit",
        r#"class Test {
    void test() {
        Map<String, Object> result = processor.process(inputData, config.getTimeout(), config.getRetryCount(), true);
    }
}
"#,
    );
}

// =============================================================================
// Category J: Blank line stability near wrapping boundaries
// =============================================================================

#[test]
fn stability_blank_line_after_wrapping_field() {
    assert_idempotent(
        "blank_line_after_wrapping_field",
        r#"class Test {
    String veryLongFieldName = someObject.someMethod().anotherMethod().yetAnother().finalMethod();

    void nextMethod() {}
}
"#,
    );
}

#[test]
fn stability_blank_line_between_methods() {
    assert_idempotent(
        "blank_line_between_methods",
        r#"class Test {
    public void firstMethod(String paramOne, String paramTwo, String paramThree, String paramFour) {
        doSomething();
    }

    public void secondMethod() {
        doSomethingElse();
    }
}
"#,
    );
}

#[test]
fn stability_blank_line_in_enum() {
    assert_idempotent(
        "blank_line_in_enum",
        r#"enum LongEnumName {
    FIRST_VERY_LONG_CONSTANT("first_value", 1),
    SECOND_VERY_LONG_CONSTANT("second_value", 2),
    THIRD_VERY_LONG_CONSTANT("third_value", 3);

    private final String value;
    private final int code;

    LongEnumName(String value, int code) {
        this.value = value;
        this.code = code;
    }
}
"#,
    );
}

#[test]
fn stability_blank_line_in_switch() {
    assert_idempotent(
        "blank_line_in_switch",
        r#"class Test {
    void test(int x) {
        switch (x) {
            case 1:
                handleFirstCase(paramOne, paramTwo, paramThree, paramFour, paramFive);
                break;

            case 2:
                handleSecondCase();
                break;
        }
    }
}
"#,
    );
}

// =============================================================================
// Additional edge cases: combinations of unstable patterns
// =============================================================================

#[test]
fn stability_chain_in_ternary() {
    assert_idempotent(
        "chain_in_ternary",
        r#"class Test {
    void test() {
        String value = config.isEnabled() ? config.getProvider().createInstance().initialize() : defaultProvider.getInstance();
    }
}
"#,
    );
}

#[test]
fn stability_binary_expr_in_method_chain() {
    assert_idempotent(
        "binary_expr_in_method_chain",
        r#"class Test {
    void test() {
        boolean isValid = StringUtils.isNotBlank(name) && validator.validate(name) && name.length() <= MAX_LENGTH;
    }
}
"#,
    );
}

#[test]
fn stability_assignment_with_long_rhs_chain() {
    assert_idempotent(
        "assignment_with_long_rhs_chain",
        r#"class Test {
    void test() {
        String formattedOutput = transformer.withConfig(config).transform(input).format(Locale.US).toString();
    }
}
"#,
    );
}

#[test]
fn stability_annotation_on_wrapped_method() {
    assert_idempotent(
        "annotation_on_wrapped_method",
        r#"class Test {
    @Transactional(readOnly = true, propagation = Propagation.REQUIRES_NEW, isolation = Isolation.READ_COMMITTED)
    public List<UserDto> findActiveUsersByDepartment(String departmentId, Pageable pageable) {
        return Collections.emptyList();
    }
}
"#,
    );
}

#[test]
fn stability_generic_method_with_chain() {
    assert_idempotent(
        "generic_method_with_chain",
        r#"class Test {
    void test() {
        List<Map.Entry<String, Integer>> sorted = map.entrySet().stream().sorted(Map.Entry.comparingByValue()).collect(Collectors.toList());
    }
}
"#,
    );
}

#[test]
fn stability_nested_new_in_arg_list() {
    assert_idempotent(
        "nested_new_in_arg_list",
        r#"class Test {
    void test() {
        registry.register(new DefaultEventHandler(new EventConfig(eventType, priority), new EventProcessor(processorFactory)));
    }
}
"#,
    );
}

// =============================================================================
// Narrower line width tests (more wrapping pressure → more instability)
// =============================================================================

#[test]
fn stability_narrow_width_method_chain() {
    let config = Configuration {
        line_width: 80,
        ..default_config()
    };
    assert_idempotent_with_config(
        "narrow_width_method_chain",
        r#"class Test {
    void test() {
        String result = factory.createBuilder().withParam(key, value).withTimeout(Duration.ofSeconds(30)).build();
    }
}
"#,
        &config,
    );
}

#[test]
fn stability_narrow_width_binary_expr() {
    let config = Configuration {
        line_width: 80,
        ..default_config()
    };
    assert_idempotent_with_config(
        "narrow_width_binary_expr",
        r#"class Test {
    void test() {
        boolean result = condition1 && condition2 && condition3 && condition4;
    }
}
"#,
        &config,
    );
}

#[test]
fn stability_narrow_width_params() {
    let config = Configuration {
        line_width: 80,
        ..default_config()
    };
    assert_idempotent_with_config(
        "narrow_width_params",
        r#"class Test {
    void process(String name, String value, int count, boolean flag) {
        // body
    }
}
"#,
        &config,
    );
}

#[test]
fn stability_narrow_width_ternary() {
    let config = Configuration {
        line_width: 80,
        ..default_config()
    };
    assert_idempotent_with_config(
        "narrow_width_ternary",
        r#"class Test {
    void test() {
        String x = condition ? longValueOne : longValueTwo;
    }
}
"#,
        &config,
    );
}
