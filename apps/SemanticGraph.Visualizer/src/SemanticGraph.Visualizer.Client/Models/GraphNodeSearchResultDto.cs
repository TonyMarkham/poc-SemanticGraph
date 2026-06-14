using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphNodeSearchResultDto(
    [property: JsonPropertyName("nodeId")] string NodeId,
    [property: JsonPropertyName("kind")] string Kind,
    [property: JsonPropertyName("displayLabel")] string DisplayLabel,
    [property: JsonPropertyName("qualifiedName")] string? QualifiedName,
    [property: JsonPropertyName("language")] string Language,
    [property: JsonPropertyName("sourceFilePath")] string? SourceFilePath);
