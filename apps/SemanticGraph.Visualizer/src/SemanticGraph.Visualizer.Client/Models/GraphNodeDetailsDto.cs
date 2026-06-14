using System.Text.Json;
using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphNodeDetailsDto(
    [property: JsonPropertyName("nodeId")] string NodeId,
    [property: JsonPropertyName("kind")] string Kind,
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("displayLabel")] string DisplayLabel,
    [property: JsonPropertyName("qualifiedName")] string? QualifiedName,
    [property: JsonPropertyName("language")] string Language,
    [property: JsonPropertyName("sourceFilePath")] string? SourceFilePath,
    [property: JsonPropertyName("startLine")] long? StartLine,
    [property: JsonPropertyName("startCol")] long? StartCol,
    [property: JsonPropertyName("endLine")] long? EndLine,
    [property: JsonPropertyName("endCol")] long? EndCol,
    [property: JsonPropertyName("selectionStartLine")] long? SelectionStartLine,
    [property: JsonPropertyName("selectionStartCol")] long? SelectionStartCol,
    [property: JsonPropertyName("containerNodeId")] string? ContainerNodeId,
    [property: JsonPropertyName("containerDisplayLabel")] string? ContainerDisplayLabel,
    [property: JsonPropertyName("firstSeenRunId")] long? FirstSeenRunId,
    [property: JsonPropertyName("lastSeenRunId")] long? LastSeenRunId,
    [property: JsonPropertyName("propertiesJson")] JsonElement PropertiesJson,
    [property: JsonPropertyName("incomingEdgeCount")] long IncomingEdgeCount,
    [property: JsonPropertyName("outgoingEdgeCount")] long OutgoingEdgeCount,
    [property: JsonPropertyName("relations")] IReadOnlyList<GraphNodeRelationSummaryDto> Relations,
    [property: JsonPropertyName("occurrences")] IReadOnlyList<GraphNodeOccurrenceDto> Occurrences);
