using Blazor.Diagrams.Core.Models;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed class SemanticGraphLinkModel : LinkModel
{
    public SemanticGraphLinkModel(
        GraphEdgeDto edge,
        SemanticGraphNodeModel source,
        SemanticGraphNodeModel target)
        : base(edge.Id, source, target)
    {
        Edge = edge;
        SourceNode = source;
        TargetNode = target;
    }

    public GraphEdgeDto Edge { get; }

    public SemanticGraphNodeModel SourceNode { get; }

    public SemanticGraphNodeModel TargetNode { get; }

    public double NormalWidth { get; private set; }

    public double SelectedWidth { get; private set; }

    public void SetWidths(double normalWidth, double selectedWidth)
    {
        NormalWidth = normalWidth;
        SelectedWidth = selectedWidth;
        Width = normalWidth;
    }

    public void ApplySelectedWidth(bool selected)
    {
        var nextWidth = selected ? SelectedWidth : NormalWidth;

        if (Math.Abs(Width - nextWidth) < 0.001)
        {
            return;
        }

        Width = nextWidth;
        Refresh();
    }
}
