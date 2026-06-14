using System.Text.Json;
using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphNodeOccurrenceDto(
    [property: JsonPropertyName("id")] long Id,
    [property: JsonPropertyName("runId")] long RunId,
    [property: JsonPropertyName("role")] string Role,
    [property: JsonPropertyName("sourceFilePath")] string SourceFilePath,
    [property: JsonPropertyName("startLine")] long StartLine,
    [property: JsonPropertyName("startCol")] long StartCol,
    [property: JsonPropertyName("endLine")] long EndLine,
    [property: JsonPropertyName("endCol")] long EndCol,
    [property: JsonPropertyName("enclosingNodeId")] string? EnclosingNodeId,
    [property: JsonPropertyName("rawJson")] JsonElement? RawJson);
