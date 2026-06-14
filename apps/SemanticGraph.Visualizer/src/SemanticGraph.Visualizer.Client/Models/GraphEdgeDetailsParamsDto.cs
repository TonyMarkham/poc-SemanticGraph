using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphEdgeDetailsParamsDto(
    [property: JsonPropertyName("edgeId")] string EdgeId);
