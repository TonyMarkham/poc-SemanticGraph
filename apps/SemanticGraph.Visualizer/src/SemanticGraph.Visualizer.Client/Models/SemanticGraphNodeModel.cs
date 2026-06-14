using Blazor.Diagrams.Core.Geometry;
using Blazor.Diagrams.Core.Models;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed class SemanticGraphNodeModel : NodeModel
{
    public SemanticGraphNodeModel(GraphNodeViewModel view, Point position, Size size)
        : base(view.Id, position)
    {
        View = view;
        Title = view.DisplayLabel;
        ControlledSize = true;
        Size = size;
        Locked = true;

        AddPort(PortAlignment.Left);
        AddPort(PortAlignment.Right);
        AddPort(PortAlignment.Top);
        AddPort(PortAlignment.Bottom);
    }

    public GraphNodeViewModel View { get; }
}
