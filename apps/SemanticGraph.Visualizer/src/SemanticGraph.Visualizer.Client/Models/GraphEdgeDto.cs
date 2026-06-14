using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphEdgeDto(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("sourceNodeId")] string SourceNodeId,
    [property: JsonPropertyName("targetNodeId")] string TargetNodeId,
    [property: JsonPropertyName("relation")] string Relation,
    [property: JsonPropertyName("confidence")] string Confidence,
    [property: JsonPropertyName("confidenceScore")] double ConfidenceScore);
