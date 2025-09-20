(method_declaration
	(modifiers
        (annotation
            name: (_) @annotation
            arguments: (_)
        )
    )
    name: (_) @name
)

(method_declaration
	(modifiers
	(marker_annotation
    	name: (_) @annotation)
    )
    name: (_) @name
    (#eq? @annotation "Test")
)