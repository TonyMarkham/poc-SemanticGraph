using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphNodeRelationSummaryDto(
    [property: JsonPropertyName("direction")] string Direction,
    [property: JsonPropertyName("relation")] string Relation,
    [property: JsonPropertyName("edgeCount")] long EdgeCount);
