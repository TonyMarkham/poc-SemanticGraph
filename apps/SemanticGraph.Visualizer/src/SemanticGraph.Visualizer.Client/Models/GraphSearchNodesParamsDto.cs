using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphSearchNodesParamsDto(
    [property: JsonPropertyName("query")] string Query,
    [property: JsonPropertyName("limit")] int Limit);
