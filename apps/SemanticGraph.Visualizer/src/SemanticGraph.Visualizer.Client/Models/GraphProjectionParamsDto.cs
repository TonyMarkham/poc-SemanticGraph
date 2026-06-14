using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphProjectionParamsDto(
    [property: JsonPropertyName("limit")] int Limit);
