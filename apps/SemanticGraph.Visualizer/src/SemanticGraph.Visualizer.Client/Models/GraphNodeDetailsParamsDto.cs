using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphNodeDetailsParamsDto(
    [property: JsonPropertyName("nodeId")] string NodeId);
