using System.Text.Json;
using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphEdgeEvidenceDto(
    [property: JsonPropertyName("id")] long Id,
    [property: JsonPropertyName("runId")] long RunId,
    [property: JsonPropertyName("provider")] string Provider,
    [property: JsonPropertyName("lspMethod")] string? LspMethod,
    [property: JsonPropertyName("sourceFilePath")] string? SourceFilePath,
    [property: JsonPropertyName("startLine")] long? StartLine,
    [property: JsonPropertyName("startCol")] long? StartCol,
    [property: JsonPropertyName("endLine")] long? EndLine,
    [property: JsonPropertyName("endCol")] long? EndCol,
    [property: JsonPropertyName("rawJson")] JsonElement? RawJson);
