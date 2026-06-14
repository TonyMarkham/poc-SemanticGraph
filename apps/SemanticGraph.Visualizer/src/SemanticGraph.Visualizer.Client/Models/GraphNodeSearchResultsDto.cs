using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphNodeSearchResultsDto(
    [property: JsonPropertyName("results")] IReadOnlyList<GraphNodeSearchResultDto> Results);
