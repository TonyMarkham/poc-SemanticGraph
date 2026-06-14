using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphNodeDto(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("kind")] string Kind,
    [property: JsonPropertyName("displayLabel")] string DisplayLabel,
    [property: JsonPropertyName("qualifiedName")] string? QualifiedName,
    [property: JsonPropertyName("language")] string Language,
    [property: JsonPropertyName("sourceFilePath")] string? SourceFilePath);
